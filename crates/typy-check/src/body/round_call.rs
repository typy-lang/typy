//! Checks `round(number)` and `round(number, ndigits)` (LIB-02).

use typy_diagnostics::{Diagnostic, DiagnosticCode, RuleId, Span};
use typy_source::syntax::{Expr, Keyword};

use super::BodyChecker;
use crate::builtin::Builtin;
use crate::typed::{TypedConstant, TypedExpr, TypedExprKind};
use crate::types::Type;

impl BodyChecker<'_> {
    /// Checks a call of `round`. An `int` rounds to an `int`; a `float` rounds to an `int`
    /// without `ndigits` and to a `float` with it, as in CPython. `ndigits` may be given by
    /// position or keyword.
    pub(super) fn call_round(
        &mut self,
        arguments: &[Expr],
        keywords: &[Keyword],
        span: Span,
    ) -> TypedExpr {
        let mut given: Vec<&Expr> = arguments.iter().collect();
        for keyword in keywords {
            match keyword.name.as_ref().map(|name| name.name.as_str()) {
                Some("ndigits") if given.len() == 1 => given.push(&keyword.value),
                name => {
                    let message = match name {
                        Some("ndigits") => "`ndigits` is given twice".to_owned(),
                        Some(other) => format!("`round` has no parameter named `{other}`"),
                        None => "`round` takes no `**` arguments".to_owned(),
                    };
                    return self.error(
                        Diagnostic::new(DiagnosticCode::error(404), keyword.span, message)
                            .with_rule(RuleId::LIB_02),
                        span,
                    );
                }
            }
        }
        let values: Vec<TypedExpr> = given.iter().map(|value| self.expression(value)).collect();
        if !(1..=2).contains(&values.len()) {
            return self.error(
                Diagnostic::new(
                    DiagnosticCode::error(404),
                    span,
                    format!(
                        "`round` takes 1 or 2 arguments, and {} {} given",
                        values.len(),
                        if values.len() == 1 { "is" } else { "are" }
                    ),
                )
                .with_rule(RuleId::LIB_02),
                span,
            );
        }
        if values.iter().any(|value| value.ty.has_error()) {
            return Self::typed(
                TypedExprKind::Constant(TypedConstant::None),
                Type::Error,
                span,
            );
        }
        if let Some(digits) = values.get(1) {
            self.expect(digits, Type::Int, "`ndigits`");
        }
        let result = match (&values[0].ty, values.len()) {
            (Type::Int, _) | (Type::Float, 1) => Type::Int,
            (Type::Float, _) => Type::Float,
            (Type::Bool, _) => self.bool_arithmetic(values[0].span),
            (other, _) => {
                let other = other.clone();
                self.operator_error("round", &other, &other, values[0].span)
            }
        };
        Self::typed(
            TypedExprKind::Builtin {
                builtin: Builtin::Round,
                arguments: values,
            },
            result,
            span,
        )
    }
}
