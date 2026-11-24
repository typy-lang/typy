//! Calls of a function or method with `@overload` signatures (TYP-29, ADR-0112): a call is
//! checked against each overload in order, and the first that takes its arguments gives the
//! types they are checked with and the call's result. The implementation runs, given each
//! argument as its own parameter holds it, and its result is read as the overload's, which
//! the body's check against that overload shows it is.

use typy_diagnostics::{Diagnostic, DiagnosticCode, RuleId, Severity, Span};
use typy_source::syntax::{Expr, Keyword, ParameterKind};

use super::BodyChecker;
use crate::check::overload_fit;
use crate::typed::{Argument, FunctionId, FunctionSignature, TypedExpr, TypedExprKind};
use crate::types::Type;

impl BodyChecker<'_> {
    /// Returns the arguments of a call of `function`, bound to its own parameters after the
    /// first `skip`, and the result the overload that takes them gives; `None` when the
    /// function has no overloads, and the error expression after reporting a call no
    /// overload takes.
    pub(super) fn overloaded_call(
        &mut self,
        function: FunctionId,
        skip: usize,
        name: &str,
        (arguments, keywords): (&[Expr], &[Keyword]),
        span: Span,
    ) -> Option<Result<(Vec<Argument>, Type), TypedExpr>> {
        let overloads = self.program.overloads.get(&function)?.clone();
        if super::call_expression::unpacking(arguments, keywords).is_some() {
            // The call's own refusal stands, as one that takes what it unpacks does.
            self.unpacking_calls.push(span);
            for argument in arguments {
                self.expression(argument);
            }
            return Some(Err(self.error(
                Diagnostic::new(
                    DiagnosticCode::error(201),
                    span,
                    "a call of a function with `@overload` signatures that unpacks with `*` \
                     or `**` is planned for TyPy 0.13 (TYP-35)",
                )
                .with_rule(RuleId::TYP_35),
                span,
            )));
        }
        let implementation = self.program.signatures[function.0 as usize].clone();
        for overload in &overloads {
            let Ok(view) = overload_fit::fit(overload, &implementation) else {
                continue;
            };
            let mark = self.diagnostics.len();
            let bound = self.bind_to(&view.signature, skip, name, arguments, keywords, span);
            let fits = self
                .diagnostics
                .iter()
                .skip(mark)
                .all(|diagnostic| diagnostic.severity() != Severity::Error);
            match bound {
                Ok(bound) if fits => {
                    let bound = retargeted(bound, &view.positions, &implementation);
                    return Some(Ok((bound, view.signature.returns.clone())));
                }
                _ => self.diagnostics.truncate(mark),
            }
        }
        for argument in arguments {
            self.expression(argument);
        }
        for keyword in keywords {
            self.expression(&keyword.value);
        }
        let shown: Vec<String> = overloads
            .iter()
            .map(|overload| format!("`{}`", written(name, overload, skip)))
            .collect();
        Some(Err(self.error(
            Diagnostic::new(
                DiagnosticCode::error(432),
                span,
                format!("no overload of `{name}` takes these arguments"),
            )
            .with_rule(RuleId::TYP_29)
            .with_help(format!("its overloads are {}", shown.join(", "))),
            span,
        )))
    }

    /// Returns the call of the overloaded method `target`, with the instance argument
    /// `bound`, read as the result of the overload that takes the arguments; `None` when the
    /// method has no overloads.
    pub(super) fn overloaded_method_call(
        &mut self,
        (target, signature): (FunctionId, &FunctionSignature),
        mut bound: Vec<Argument>,
        name: &str,
        given: (&[Expr], &[Keyword]),
        span: Span,
    ) -> Option<TypedExpr> {
        let (arguments, result) = match self.overloaded_call(target, 1, name, given, span)? {
            Ok(found) => found,
            Err(error) => return Some(error),
        };
        bound.extend(arguments);
        let call = Self::typed(
            TypedExprKind::Call {
                function: target,
                arguments: bound,
                dispatch: true,
            },
            signature.returns.clone(),
            span,
        );
        Some(Self::overload_result(call, &result))
    }

    /// Returns a call of an overloaded function read as the overload's result. The check of
    /// the body against the overload shows the implementation returns one, and a test when
    /// the program runs keeps a value of another type from being read as one, raising
    /// `TypeError` instead, so a mistake in what a type claims never reads memory wrongly.
    pub(super) fn overload_result(call: TypedExpr, result: &Type) -> TypedExpr {
        let wanted = result.value_type();
        if call.ty.has_error() || call.ty == wanted {
            return call;
        }
        let span = call.span;
        let kept = call.clone();
        Self::tested_as(call, &wanted, span).unwrap_or(kept)
    }
}

/// Returns arguments bound to an overload's view as arguments of the implementation: each at
/// the implementation's position of its parameter, held as that parameter holds it.
fn retargeted(
    bound: Vec<Argument>,
    positions: &[usize],
    implementation: &FunctionSignature,
) -> Vec<Argument> {
    bound
        .into_iter()
        .map(|argument| {
            let at = positions[argument.parameter];
            let own = &implementation.parameters[at];
            let held = match (&own.kind, &own.ty) {
                (ParameterKind::VarPositional, Type::TupleOf(item)) => item.as_ref(),
                (ParameterKind::VarKeyword, Type::Dict(_, value)) => value.as_ref(),
                (_, ty) => ty,
            };
            Argument {
                parameter: at,
                passed: argument.passed,
                value: BodyChecker::converted(argument.value, Some(held)),
            }
        })
        .collect()
}

/// Returns an overload as a call of it is written, `name(x: int) -> int`, without the
/// parameters the instance fills.
fn written(name: &str, overload: &FunctionSignature, skip: usize) -> String {
    let parameters: Vec<String> = overload.parameters[skip.min(overload.parameters.len())..]
        .iter()
        .map(|parameter| format!("{}: {}", parameter.name, parameter.ty))
        .collect();
    format!("{name}({}) -> {}", parameters.join(", "), overload.returns)
}
