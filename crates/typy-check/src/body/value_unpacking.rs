//! A call of a function value with `*` (EXP-22, ADR-0083): the values before the first `*`
//! bind to the `Callable`'s first parameters, and every value from it on goes into one list,
//! whose length the program checks when it runs against the parameters left, raising
//! CPython's `TypeError`, which names the function the value holds.

use typy_diagnostics::{Diagnostic, DiagnosticCode, RuleId, Span};
use typy_source::syntax::Expr;

use super::BodyChecker;
use super::spread::held_alike;
use crate::typed::{SpreadKind, SpreadPart, TypedExpr, TypedExprKind};
use crate::types::Type;

impl BodyChecker<'_> {
    /// Returns `callee(arguments)` of the function value `callee`, whose `Callable` takes
    /// `parameters` and gives `returns`, when an argument is `*iterable`.
    pub(super) fn call_value_unpacked(
        &mut self,
        callee: TypedExpr,
        (parameters, returns): (Vec<Type>, Type),
        arguments: &[Expr],
        span: Span,
    ) -> TypedExpr {
        // The call takes what it unpacks, so its own errors stand (ADR-0073).
        self.unpacking_calls.push(span);
        let first = arguments
            .iter()
            .position(|argument| matches!(argument, Expr::Starred { .. }))
            .unwrap_or(arguments.len());
        if first > parameters.len() {
            for argument in arguments {
                self.expression(argument);
            }
            let message = format!(
                "a value of type {} takes {} argument{}, and more are given",
                callee.ty,
                parameters.len(),
                if parameters.len() == 1 { "" } else { "s" },
            );
            return self.error(
                Diagnostic::new(DiagnosticCode::error(404), span, message)
                    .with_rule(RuleId::FUN_05),
                span,
            );
        }
        let fixed = arguments[..first]
            .iter()
            .zip(&parameters)
            .enumerate()
            .map(|(position, (argument, parameter))| {
                let value = self.expression_expecting(argument, Some(parameter));
                self.expect(
                    &value,
                    parameter.clone(),
                    &format!("argument {}", position + 1),
                );
                Self::converted(value, Some(parameter))
            })
            .collect();
        let rest = &parameters[first..];
        // The values from the first `*` on go into one list, of the one type the parameters
        // they may reach take.
        let item = rest
            .split_first()
            .filter(|(one, others)| others.iter().all(|other| other == *one))
            .map(|(one, _)| one.clone());
        if item.is_none() && !rest.is_empty() {
            let message = format!(
                "`*` in a call of a value of type {} may give a value to parameters of more \
                 than one type, which one list cannot hold",
                callee.ty
            );
            self.diagnostics.push(
                Diagnostic::new(DiagnosticCode::error(404), span, message)
                    .with_rule(RuleId::EXP_22)
                    .with_help("pass the values one by one"),
            );
        }
        let list = self.unpacked_values(&arguments[first..], item.as_ref(), span);
        Self::typed(
            TypedExprKind::CallValueUnpacked {
                callee: Box::new(callee),
                fixed,
                rest: Box::new(list),
            },
            returns,
            span,
        )
    }

    /// Returns the list of the values `arguments` give, the first `*` of a call and those
    /// after it, each of type `item` when the parameters they may reach take one.
    fn unpacked_values(
        &mut self,
        arguments: &[Expr],
        item: Option<&Type>,
        span: Span,
    ) -> TypedExpr {
        let mut parts = Vec::new();
        let mut held = item.cloned();
        for argument in arguments {
            let Expr::Starred { value, .. } = argument else {
                let value = self.expression_expecting(argument, item);
                if let Some(item) = item {
                    self.expect(&value, item.clone(), "a value after `*`");
                }
                held.get_or_insert_with(|| value.ty.clone());
                parts.push(SpreadPart::Item(Self::converted(value, item)));
                continue;
            };
            let value = self.tuple_expression(value);
            let at = value.span;
            let (source, given) = self.sequence_source(Self::sequence_of(value), "`*`");
            if let Some(item) = item
                && !given.has_error()
                && !item.has_error()
                && !held_alike(&given, item)
            {
                let message =
                    format!("`*` gives values of type {given}, and the parameters take {item}");
                self.diagnostics.push(
                    Diagnostic::new(DiagnosticCode::error(404), at, message)
                        .with_rule(RuleId::EXP_22),
                );
            }
            held.get_or_insert(given);
            parts.push(SpreadPart::Each(Box::new(source)));
        }
        Self::typed(
            TypedExprKind::Spread {
                kind: SpreadKind::List,
                parts,
            },
            Type::List(Box::new(held.unwrap_or(Type::Error))),
            span,
        )
    }
}
