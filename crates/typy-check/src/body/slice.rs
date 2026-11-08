//! Checks slices of strings and lists, `value[lower:upper:step]` (DAT-12).

use typy_diagnostics::{Diagnostic, DiagnosticCode, RuleId, Span};
use typy_source::syntax::Expr;

use super::BodyChecker;
use crate::typed::{TypedConstant, TypedExpr, TypedExprKind};
use crate::types::Type;

/// The three optional bounds of a slice, as written.
pub(super) struct Bounds<'source> {
    /// The start.
    pub(super) lower: Option<&'source Expr>,
    /// The stop.
    pub(super) upper: Option<&'source Expr>,
    /// The step.
    pub(super) step: Option<&'source Expr>,
}

impl BodyChecker<'_> {
    /// Checks `value[lower:upper:step]`: each bound is an `int`, and the slice is a new value of
    /// the sliced type.
    pub(super) fn slice(&mut self, value: TypedExpr, bounds: &Bounds<'_>, span: Span) -> TypedExpr {
        let bound = |checker: &mut Self, bound: Option<&Expr>| {
            bound.map(|bound| {
                let bound = checker.expression(bound);
                checker.expect(&bound, Type::Int, "a slice bound");
                Box::new(bound)
            })
        };
        let lower = bound(self, bounds.lower);
        let upper = bound(self, bounds.upper);
        let step = bound(self, bounds.step);
        match &value.ty {
            Type::Str | Type::List(_) => {
                let ty = value.ty.clone();
                Self::typed(
                    TypedExprKind::Slice {
                        value: Box::new(value),
                        lower,
                        upper,
                        step,
                    },
                    ty,
                    span,
                )
            }
            Type::Error => Self::typed(
                TypedExprKind::Constant(TypedConstant::None),
                Type::Error,
                span,
            ),
            Type::Tuple(_) => self.error(
                Diagnostic::new(
                    DiagnosticCode::error(201),
                    span,
                    "slices of a tuple are planned for TyPy 0.2 (DAT-12)",
                )
                .with_rule(RuleId::DAT_12),
                span,
            ),
            other => self.error(
                Diagnostic::new(
                    DiagnosticCode::error(401),
                    value.span,
                    format!("{} cannot be sliced", other.with_article()),
                )
                .with_rule(RuleId::DAT_12),
                span,
            ),
        }
    }

    /// Reports a slice that is not the whole index, such as `grid[1:2, 0]`, which indexes with a
    /// tuple of a slice and an `int`.
    pub(super) fn misplaced_slice(&mut self, span: Span) -> TypedExpr {
        self.error(
            Diagnostic::new(
                DiagnosticCode::error(401),
                span,
                "a slice is the whole index, and an index that holds one is not part of TyPy",
            )
            .with_rule(RuleId::DAT_12),
            span,
        )
    }
}
