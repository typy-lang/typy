//! Checks slices of strings and lists, `value[lower:upper:step]` (DAT-12).

use typy_diagnostics::{Diagnostic, DiagnosticCode, RuleId, Span};
use typy_source::syntax::Expr;

use super::BodyChecker;
use super::expression::literal_position;
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
            Type::Str | Type::Bytes | Type::List(_) | Type::TupleOf(_) | Type::Range => {
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
            Type::Tuple(items) => {
                let items = items.clone();
                self.tuple_slice(value, &items, bounds, span)
            }
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

    /// Checks `tuple[lower:upper:step]`: each bound is an integer literal or missing, so the
    /// items selected, and the type of the new tuple, are known when compiling (DAT-08).
    fn tuple_slice(
        &mut self,
        tuple: TypedExpr,
        items: &[Type],
        bounds: &Bounds<'_>,
        span: Span,
    ) -> TypedExpr {
        let literal = |bound: Option<&Expr>| {
            bound.map_or(Some(None), |bound| literal_position(bound).map(Some))
        };
        let (Some(lower), Some(upper), Some(step)) = (
            literal(bounds.lower),
            literal(bounds.upper),
            literal(bounds.step),
        ) else {
            return self.error(
                Diagnostic::new(
                    DiagnosticCode::error(307),
                    span,
                    "a tuple is sliced with integer literals, because each item can have its own type",
                )
                .with_rule(RuleId::DAT_08)
                .with_help("write the bounds as numbers, such as `pair[1:]`"),
                span,
            );
        };
        let Some(positions) = positions(lower, upper, step, items.len()) else {
            return self.error(
                Diagnostic::new(
                    DiagnosticCode::error(307),
                    span,
                    "a slice step cannot be zero, so this slice always raises ValueError",
                )
                .with_rule(RuleId::DAT_12),
                span,
            );
        };
        let ty = Type::Tuple(
            positions
                .iter()
                .map(|position| items[*position].clone())
                .collect(),
        );
        Self::typed(
            TypedExprKind::TupleSlice {
                tuple: Box::new(tuple),
                positions,
            },
            ty,
            span,
        )
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

/// Returns the positions `[lower:upper:step]` selects in `length` items, as CPython's
/// `PySlice_AdjustIndices` gives them, or `None` for a zero step.
fn positions(
    lower: Option<i128>,
    upper: Option<i128>,
    step: Option<i128>,
    length: usize,
) -> Option<Vec<usize>> {
    let step = step.unwrap_or(1);
    if step == 0 {
        return None;
    }
    let length = i128::try_from(length).unwrap_or(i128::MAX);
    let backwards = step < 0;
    let adjust = |bound: i128| {
        let bound = if bound < 0 { bound + length } else { bound };
        if backwards {
            bound.clamp(-1, length - 1)
        } else {
            bound.clamp(0, length)
        }
    };
    let start = lower.map_or(if backwards { length - 1 } else { 0 }, adjust);
    let end = upper.map_or(if backwards { -1 } else { length }, adjust);
    let mut selected = Vec::new();
    let mut position = start;
    while (step > 0 && position < end) || (step < 0 && position > end) {
        selected.push(usize::try_from(position).unwrap_or(0));
        position += step;
    }
    Some(selected)
}

#[cfg(test)]
mod tests {
    use super::positions;

    /// A slice's lower bound, upper bound and step, `None` standing for a missing one.
    type Bounds = (Option<i128>, Option<i128>, Option<i128>);

    #[test]
    fn tuple_slices_select_what_cpython_selects() {
        let cases: [(Bounds, Option<&[usize]>); 8] = [
            ((None, None, None), Some(&[0, 1, 2, 3])),
            ((Some(1), None, None), Some(&[1, 2, 3])),
            ((None, None, Some(-1)), Some(&[3, 2, 1, 0])),
            ((Some(-2), Some(100), None), Some(&[2, 3])),
            ((Some(9), Some(-9), Some(-2)), Some(&[3, 1])),
            ((Some(3), Some(1), None), Some(&[])),
            ((Some(-100), Some(1), None), Some(&[0])),
            ((None, None, Some(0)), None),
        ];
        for ((lower, upper, step), expected) in cases {
            assert_eq!(
                positions(lower, upper, step, 4).as_deref(),
                expected,
                "{lower:?}:{upper:?}:{step:?}"
            );
        }
    }
}
