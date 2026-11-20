//! Reads a built-in function as a value (FUN-08, ADR-0077): a hidden function of the
//! `Callable` it fills, whose body calls the built-in, made once for each built-in and
//! `Callable` type of a module. Its object is written as CPython writes a built-in function.

use typy_diagnostics::{Diagnostic, DiagnosticCode, Diagnostics, RuleId, Span};
use typy_source::syntax::{Expr, ParameterKind, Stmt};

use super::BodyChecker;
use crate::builtin::Builtin;
use crate::check::Definition;
use crate::typed::{
    FunctionId, FunctionSignature, SignatureParameter, TypedConstant, TypedExpr, TypedExprKind,
};
use crate::types::Type;

impl BodyChecker<'_> {
    /// Returns the built-in function `expression` names as a value of the `Callable` whose
    /// parameters are `parameters` and whose result is `returns`, when it names one the
    /// program does not bind, or `None` for any other expression.
    pub(super) fn builtin_as_callable(
        &mut self,
        expression: &Expr,
        parameters: &[Type],
        returns: Option<&Type>,
    ) -> Option<TypedExpr> {
        let Expr::Name { id, span } = expression else {
            return None;
        };
        let builtin = self.unbound_builtin_function(id, *span)?;
        Some(self.builtin_value(builtin, *span, parameters, returns))
    }

    /// Returns the built-in function `id` names where the program binds nothing of that name.
    pub(super) fn unbound_builtin_function(&self, id: &str, span: Span) -> Option<Builtin> {
        let builtin = Builtin::named(id).filter(|builtin| builtin.is_function())?;
        let bound = self.locals.contains_key(id)
            || self.closure_read(id, span).is_some()
            || self.program.module_symbols.contains_key(id)
            || self.program.function_ids.contains_key(id)
            || self.program.class_ids.contains_key(id)
            || self.program.module_functions.contains_key(id);
        (!bound).then_some(builtin)
    }

    /// Returns `builtin` as a value where no `Callable` is expected: of its one type for a
    /// built-in of one signature, such as `chr`, and E0203 for one whose arguments may be of
    /// several types, which a `Callable` type of the variable chooses.
    pub(super) fn builtin_without_context(&mut self, builtin: Builtin, span: Span) -> TypedExpr {
        let fixed = match builtin {
            Builtin::Chr => Some((Type::Int, Type::Str)),
            Builtin::Ord => Some((Type::Str, Type::Int)),
            _ => None,
        };
        if let Some((parameter, result)) = fixed {
            return self.builtin_value(builtin, span, &[parameter], Some(&result));
        }
        let name = builtin.name();
        self.error(
            Diagnostic::new(
                DiagnosticCode::error(203),
                span,
                format!(
                    "`{name}` takes arguments of more than one type, so as a value it needs the \
                     `Callable` type its context expects"
                ),
            )
            .with_rule(RuleId::FUN_08)
            .with_help(format!(
                "annotate the variable with the types you call it with, such as \
                 `measure: Callable[[str], int] = {name}`"
            )),
            span,
        )
    }

    /// Returns the hidden function of `builtin` for the `Callable` of `parameters` and
    /// `returns`, made the first time the module needs it, as a value. A call the built-in
    /// refuses is reported at `span`, and a result that does not pass unchanged as `returns`
    /// is E0404 (NFR-19).
    fn builtin_value(
        &mut self,
        builtin: Builtin,
        span: Span,
        parameters: &[Type],
        returns: Option<&Type>,
    ) -> TypedExpr {
        let name = builtin.name();
        let function = match self.program.builtin_value(name, parameters) {
            Some(function) => function,
            None => match self.make_builtin_value(name, span, parameters) {
                Some(function) => function,
                None => {
                    return Self::typed(
                        TypedExprKind::Constant(TypedConstant::None),
                        Type::Error,
                        span,
                    );
                }
            },
        };
        let result = self.program.signatures[function.0 as usize].returns.clone();
        if let Some(wanted) = returns
            && !result.matches(wanted)
        {
            return self.error(
                Diagnostic::new(
                    DiagnosticCode::error(404),
                    span,
                    format!(
                        "`{name}` called with {} gives {}, and this `Callable` returns {wanted}",
                        argument_types(parameters),
                        result.with_article()
                    ),
                )
                .with_rule(RuleId::FUN_08),
                span,
            );
        }
        let ty = Type::Callable {
            parameters: parameters.to_vec(),
            returns: Box::new(result),
        };
        Self::typed(TypedExprKind::FunctionValue(function), ty, span)
    }

    /// Checks the hidden function `return name(value0, ...)` of `parameters`, whose result
    /// takes the call's type, and keeps it for the module; reports what the call refuses at
    /// `span` and returns `None` then.
    fn make_builtin_value(
        &mut self,
        name: &str,
        span: Span,
        parameters: &[Type],
    ) -> Option<FunctionId> {
        let function = self.program.next_function();
        let names: Vec<String> = (0..parameters.len())
            .map(|index| format!("value{index}"))
            .collect();
        let signature = FunctionSignature {
            parameters: names
                .iter()
                .zip(parameters)
                .map(|(each, ty)| SignatureParameter {
                    name: each.clone(),
                    ty: ty.clone(),
                    default: None,
                    default_call: None,
                    kind: ParameterKind::PositionalOnly,
                })
                .collect(),
            // The result takes the call's type once the body is checked.
            returns: Type::Error,
        };
        self.program.signatures.push(signature.clone());
        self.program.functions.push(None);
        let call = Expr::Call {
            function: Box::new(Expr::Name {
                id: name.to_owned(),
                span,
            }),
            arguments: names
                .iter()
                .map(|each| Expr::Name {
                    id: each.clone(),
                    span,
                })
                .collect(),
            keywords: Vec::new(),
            span,
        };
        let statements = [Stmt::Return {
            value: Some(call),
            span,
        }];
        let definition = Definition {
            name: name.to_owned(),
            qualified_name: name.to_owned(),
            class: None,
            takes_instance: false,
            copy_of: None,
            home: None,
            signature,
            type_parameters: Vec::new(),
            parameters: names.iter().map(|each| (each.clone(), span)).collect(),
            body: &statements,
            span,
        };
        let mut found = Diagnostics::new();
        Self::check_body_of(self.program, function, &definition, None, &mut found);
        self.settle_lambda_result(function);
        if !found.is_empty() {
            for diagnostic in found.iter() {
                self.diagnostics.push(diagnostic.clone());
            }
            return None;
        }
        self.program
            .add_builtin_value(name, parameters.to_vec(), function);
        Some(function)
    }
}

/// Returns the types of a call's arguments as a message writes them: `an int and a str`.
fn argument_types(parameters: &[Type]) -> String {
    match parameters {
        [] => "no arguments".to_owned(),
        [one] => one.with_article(),
        [first @ .., last] => format!(
            "{} and {}",
            first
                .iter()
                .map(Type::with_article)
                .collect::<Vec<_>>()
                .join(", "),
            last.with_article()
        ),
    }
}
