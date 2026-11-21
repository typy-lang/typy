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
    /// and the result of the type `expected` gives, or of the body's type. A parameter
    /// without a type is E0203, and one taken by keyword only is E0202.
    pub(super) fn lambda(&mut self, expression: &Expr, expected: Expected<'_>) -> TypedExpr {
        let Expr::Lambda {
            parameters,
            body,
            span,
        } = expression
        else {
            // A built-in function takes the `Callable` the context expects (FUN-08).
            if let Some(types) = expected.parameters
                && let Some(value) = self.builtin_as_callable(expression, types, expected.returns)
            {
                return value;
            }
            // So does a method read through a built-in value, such as `words.append`.
            if let (Some(types), Expr::Attribute { span, .. }) = (expected.parameters, expression) {
                self.expected_method = Some((*span, types.to_vec(), expected.returns.cloned()));
                let value = self.expression(expression);
                self.expected_method = None;
                return value;
            }
            return self.expression(expression);
        };
        if let Some(refused) = self.refused_lambda(*span) {
            return refused;
        }
        let Some(types) = self.lambda_parameter_types(parameters, expected.parameters) else {
            return Self::typed(
                TypedExprKind::Constant(TypedConstant::None),
                Type::Error,
                *span,
            );
        };
        let function = self.program.next_function();
        let signature = FunctionSignature {
            parameters: parameters
                .iter()
                .zip(&types)
                .map(|(parameter, ty)| SignatureParameter {
                    name: parameter.name.name.clone(),
                    ty: ty.clone(),
                    default: None,
                    default_call: None,
                    kind: parameter.kind,
                })
                .collect(),
            // A result no context gives takes the body's type once the body is checked.
            returns: expected.returns.cloned().unwrap_or(Type::Error),
        };
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
            returns: Box::new(returns),
        };
        Self::typed(TypedExprKind::Lambda { function, captures }, ty, *span)
    }

    /// Returns E0306 for a `lambda` that captures a variable assigned no type yet: its body is
    /// checked where it stands.
    fn refused_lambda(&mut self, span: Span) -> Option<TypedExpr> {
        let untyped = self.captured_names(span).into_iter().find(|name| {
            self.cell_symbol(name)
                .is_some_and(|symbol| !self.types.contains_key(&symbol))
        })?;
        let diagnostic = Diagnostic::new(
            DiagnosticCode::error(306),
            span,
            format!("this `lambda` reads `{untyped}`, which has no type yet where it stands"),
        )
        .with_rule(RuleId::TYP_12)
        .with_help(format!(
            "assign or annotate `{untyped}` before the `lambda`"
        ));
        Some(self.error(diagnostic, span))
    }

    /// Returns the type of each parameter of a `lambda`, from the types `expected` gives by
    /// position, or `None` after reporting a parameter without one.
    fn lambda_parameter_types(
        &mut self,
        parameters: &[Parameter],
        expected: Option<&[Type]>,
    ) -> Option<Vec<Type>> {
        if let Some(parameter) = parameters
            .iter()
            .find(|parameter| parameter.kind == ParameterKind::KeywordOnly)
        {
            let name = &parameter.name.name;
            self.diagnostics.push(
                Diagnostic::new(
                    DiagnosticCode::error(202),
                    parameter.name.span,
                    format!(
                        "the `lambda` takes `{name}` by keyword only, and a `Callable` passes \
                         every argument by position"
                    ),
                )
                .with_rule(RuleId::FUN_05),
            );
            return None;
        }
        let given = expected.unwrap_or_default();
        if let Some(parameter) = parameters.get(given.len()) {
            let message = match expected {
                Some(_) => format!(
                    "`lambda` parameter `{}` has no type: the `Callable` expected here passes {} \
                     argument{}",
                    parameter.name.name,
                    given.len(),
                    if given.len() == 1 { "" } else { "s" }
                ),
                None => format!(
                    "`lambda` parameter `{}` has no type: a `lambda` takes the types of its \
                     parameters from the `Callable` its context expects",
                    parameter.name.name
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
            return None;
        }
        Some(given[..parameters.len()].to_vec())
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
