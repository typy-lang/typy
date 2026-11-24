//! Checks `lambda` expressions (EXP-14, ADR-0060, ADR-0061): a function of one expression,
//! made each time the expression runs, whose parameters take their types from the
//! `Callable` the context expects. Its body is checked where it stands, with the types its
//! captured variables have there, so a `lambda` whose result no context gives, such as a
//! `key`, takes the type of its body.

use typy_diagnostics::{Diagnostic, DiagnosticCode, RuleId, Span};
use typy_source::syntax::{Expr, Parameter, ParameterKind, Stmt};

use super::BodyChecker;
use crate::check::Definition;
use crate::typed::{
    FunctionSignature, SignatureParameter, TypedConstant, TypedExpr, TypedExprKind, TypedStmt,
};
use crate::types::Type;

/// What the context of a `lambda` expects of it: the types of its parameters, by position,
/// and of its result, when it gives them.
#[derive(Clone, Copy, Default)]
pub(super) struct Expected<'types> {
    pub(super) parameters: Option<&'types [Type]>,
    pub(super) returns: Option<&'types Type>,
}

impl BodyChecker<'_> {
    /// Returns the function object the `lambda` `expression` makes, of type
    /// `Callable[[A, ...], R]`: each parameter of the type `expected` gives at its position,
    /// and the result of the type `expected` gives, or of the body's type. One whose
    /// parameters are not the `Callable`'s one for one, with defaults or `*args`, is the
    /// adapter of its function object (FUN-19). A parameter without a type is E0203.
    pub(super) fn lambda(&mut self, expression: &Expr, expected: Expected<'_>) -> TypedExpr {
        let Expr::Lambda {
            parameters,
            body,
            span,
        } = expression
        else {
            return self.expected_value(expression, expected);
        };
        if let Some(refused) = self.refused_capture(*span, "`lambda`") {
            return refused;
        }
        let Some(parameter_types) = self.lambda_parameter_types(parameters, expected.parameters)
        else {
            return Self::typed(
                TypedExprKind::Constant(TypedConstant::None),
                Type::Error,
                *span,
            );
        };
        let function = self.program.next_function();
        let types: Vec<Type> = parameter_types.iter().map(|(ty, _)| ty.clone()).collect();
        let signature = FunctionSignature {
            parameters: parameters
                .iter()
                .zip(parameter_types)
                .map(|(parameter, (ty, default))| SignatureParameter {
                    name: parameter.name.name.clone(),
                    ty,
                    default,
                    default_call: None,
                    kind: parameter.kind,
                })
                .collect(),
            // A result no context gives takes the body's type once the body is checked.
            returns: expected.returns.cloned().unwrap_or(Type::Error),
        };
        // A `lambda` whose parameters are not the `Callable`'s one for one takes its arguments
        // through an adapter (FUN-19).
        let adapted = expected
            .parameters
            .is_some_and(|given| super::callable_adapter::needs_adapter(&signature, 0, given));
        self.program.signatures.push(signature.clone());
        self.program.functions.push(None);
        // A module variable is read as it is, so only a comprehension's variable is captured
        // at module level.
        let mut names = self.captured_names(*span);
        names.retain(|name| {
            self.locals
                .get(name)
                .is_none_or(|symbol| !self.is_module_variable(*symbol))
        });
        let (closure, captures) = self.capture_values("<lambda>", function, &names, *span);
        let qualified_name = if self.qualified_name.is_empty() {
            "<lambda>".to_owned()
        } else {
            format!("{}.<locals>.<lambda>", self.qualified_name)
        };
        let statements = [Stmt::Return {
            value: Some(body.as_ref().clone()),
            span: body.span(),
        }];
        let definition = Definition {
            name: "<lambda>".to_owned(),
            qualified_name,
            class: None,
            takes_instance: false,
            copy_of: None,
            home: None,
            signature,
            type_parameters: Vec::new(),
            parameters: parameters
                .iter()
                .map(|parameter| (parameter.name.name.clone(), parameter.name.span))
                .collect(),
            body: &statements,
            span: *span,
        };
        self.program.closures.insert(function, closure.variables());
        Self::check_body_of(
            self.program,
            function,
            &definition,
            Some(closure),
            self.diagnostics,
        );
        let returns = match expected.returns {
            Some(returns) => returns.clone(),
            None => self.settle_lambda_result(function),
        };
        let ty = Type::Callable {
            parameters: types,
            returns: Box::new(returns.clone()),
        };
        let value = Self::typed(TypedExprKind::Lambda { function, captures }, ty, *span);
        match expected.parameters {
            Some(given) if adapted => {
                self.closure_adapter(value, (function, "<lambda>"), (given, Some(&returns)))
            }
            _ => value,
        }
    }

    /// Returns `expression`, which is no `lambda`, as a value of the `Callable` `expected`
    /// gives, when it gives one: a nested function, a built-in or a method through an adapter
    /// or a hidden function where one is needed (FUN-08, FUN-19).
    fn expected_value(&mut self, expression: &Expr, expected: Expected<'_>) -> TypedExpr {
        let Some(types) = expected.parameters else {
            return self.expression(expression);
        };
        if let Expr::Name { id, span } = expression
            && let Some(value) = self.adapted_nested_function(id, *span, (types, expected.returns))
        {
            return value;
        }
        if let Some(value) = self.builtin_as_callable(expression, types, expected.returns) {
            return value;
        }
        // The read of the object of a method may expect a `Callable` of its own.
        if let Expr::Attribute { span, .. } = expression {
            let outer =
                self.expected_method
                    .replace((*span, types.to_vec(), expected.returns.cloned()));
            let value = self.expression(expression);
            self.expected_method = outer;
            return value;
        }
        self.expression(expression)
    }

    /// Returns E0306 for a `lambda` or a generator expression, `what`, that captures a
    /// variable assigned no type yet: its body is checked where it stands (TYP-12).
    pub(super) fn refused_capture(&mut self, span: Span, what: &str) -> Option<TypedExpr> {
        let untyped = self.captured_names(span).into_iter().find(|name| {
            self.cell_symbol(name)
                .is_some_and(|symbol| !self.types.contains_key(&symbol))
        })?;
        let diagnostic = Diagnostic::new(
            DiagnosticCode::error(306),
            span,
            format!("this {what} reads `{untyped}`, which has no type yet where it stands"),
        )
        .with_rule(RuleId::TYP_12)
        .with_help(format!("assign or annotate `{untyped}` before the {what}"));
        Some(self.error(diagnostic, span))
    }

    /// Returns the type of each parameter of a `lambda`, with its default where no position of
    /// the `Callable` expected reaches it, or `None` after reporting a parameter without one:
    /// a parameter by position takes the type `expected` gives at its position, or else its
    /// default's; `*args` the type of the positions past the others; and a keyword-only
    /// parameter its default's (FUN-19, ADR-0080).
    fn lambda_parameter_types(
        &mut self,
        parameters: &[Parameter],
        expected: Option<&[Type]>,
    ) -> Option<Vec<(Type, Option<TypedConstant>)>> {
        let given = expected.unwrap_or_default();
        let positional = parameters
            .iter()
            .filter(|parameter| {
                matches!(
                    parameter.kind,
                    ParameterKind::PositionalOnly | ParameterKind::Normal
                )
            })
            .count();
        let mut found = Vec::with_capacity(parameters.len());
        for (index, parameter) in parameters.iter().enumerate() {
            let typed = match parameter.kind {
                ParameterKind::PositionalOnly | ParameterKind::Normal => match given.get(index) {
                    Some(ty) => Some((ty.clone(), None)),
                    None => self.lambda_default(parameter, expected),
                },
                ParameterKind::KeywordOnly => self.lambda_default(parameter, expected),
                ParameterKind::VarPositional => {
                    let past = given.get(positional..).unwrap_or_default();
                    self.lambda_tail(parameter, past, expected.is_some())
                }
                ParameterKind::VarKeyword => {
                    let name = &parameter.name.name;
                    let message = format!(
                        "`lambda` parameter `**{name}` has no type: a `Callable` passes no \
                         keywords"
                    );
                    self.diagnostics.push(
                        Diagnostic::new(DiagnosticCode::error(203), parameter.name.span, message)
                            .with_rule(RuleId::FUN_19)
                            .with_help(format!(
                                "define a function with `def` that annotates it, such as \
                                 `**{name}: int`"
                            )),
                    );
                    None
                }
            };
            found.push(typed?);
        }
        Some(found)
    }

    /// Returns the type of `parameter`, which no position of the `Callable` expected reaches,
    /// and its default: the default's type, as a call that leaves the argument out gives it;
    /// `None` after reporting one without a default.
    fn lambda_default(
        &mut self,
        parameter: &Parameter,
        expected: Option<&[Type]>,
    ) -> Option<(Type, Option<TypedConstant>)> {
        let name = &parameter.name.name;
        if let Some(default) = &parameter.default {
            let value = self.expression(default);
            if let TypedExprKind::Constant(constant) = value.kind {
                return Some((value.ty, Some(constant)));
            }
        }
        if parameter.kind == ParameterKind::KeywordOnly {
            self.diagnostics.push(
                Diagnostic::new(
                    DiagnosticCode::error(202),
                    parameter.name.span,
                    format!(
                        "the `lambda` takes `{name}` by keyword only, and a `Callable` passes \
                         every argument by position"
                    ),
                )
                .with_rule(RuleId::FUN_05)
                .with_help(format!("give `{name}` a default")),
            );
            return None;
        }
        let given = expected.unwrap_or_default().len();
        let message = match expected {
            Some(_) => format!(
                "`lambda` parameter `{name}` has no type: the `Callable` expected here passes \
                 {given} argument{}",
                if given == 1 { "" } else { "s" }
            ),
            None => format!(
                "`lambda` parameter `{name}` has no type: a `lambda` takes the types of its \
                 parameters from the `Callable` its context expects"
            ),
        };
        self.diagnostics.push(
            Diagnostic::new(DiagnosticCode::error(203), parameter.name.span, message)
                .with_rule(RuleId::EXP_14)
                .with_help(
                    "give it a context that expects a `Callable`, such as \
                     `scale: Callable[[int], int] = lambda x: x * 2`, or define a function \
                     with `def`",
                ),
        );
        None
    }

    /// Returns the type of `*args` of a `lambda`, whose items are the arguments `past`, the
    /// positions of the `Callable` expected past the parameters by position, which have one
    /// type; `None` after reporting none or two types.
    fn lambda_tail(
        &mut self,
        parameter: &Parameter,
        past: &[Type],
        expected: bool,
    ) -> Option<(Type, Option<TypedConstant>)> {
        let name = &parameter.name.name;
        let Some(first) = past.first() else {
            let message = if expected {
                format!(
                    "`lambda` parameter `*{name}` has no type: the `Callable` expected here \
                     passes no argument past the others"
                )
            } else {
                format!(
                    "`lambda` parameter `*{name}` has no type: a `lambda` takes the types of its \
                     parameters from the `Callable` its context expects"
                )
            };
            self.diagnostics.push(
                Diagnostic::new(DiagnosticCode::error(203), parameter.name.span, message)
                    .with_rule(RuleId::FUN_19)
                    .with_help("define a function with `def` that annotates it"),
            );
            return None;
        };
        if let Some(other) = past.iter().find(|each| *each != first) {
            let message = format!(
                "the items of `*{name}` have one type; the `Callable` expected here passes {} \
                 and {} to it",
                first.with_article(),
                other.with_article()
            );
            self.diagnostics.push(
                Diagnostic::new(DiagnosticCode::error(409), parameter.name.span, message)
                    .with_rule(RuleId::FUN_19),
            );
            return None;
        }
        Some((Type::TupleOf(Box::new(first.clone())), None))
    }

    /// Returns the type of the value the last statement of `function` returns, the body of a
    /// `lambda` or of a hidden function of a built-in, which no context gave, and gives its
    /// signature that result.
    pub(super) fn settle_lambda_result(&mut self, function: crate::typed::FunctionId) -> Type {
        let index = function.0 as usize;
        let returns = self.program.functions[index]
            .as_ref()
            .and_then(|typed| match typed.body.last() {
                Some(TypedStmt::Return {
                    value: Some(value), ..
                }) => Some(value.ty.clone()),
                _ => None,
            })
            .unwrap_or(Type::Error);
        self.program.signatures[index].returns = returns.clone();
        if let Some(typed) = &mut self.program.functions[index] {
            typed.signature.returns = returns.clone();
        }
        returns
    }
}
