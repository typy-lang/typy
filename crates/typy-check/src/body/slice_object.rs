//! `slice` objects (CLS-23): `x[a:b:c]` on an instance passes one to its class's
//! `__getitem__`, `__setitem__` or `__delitem__`; `slice(a, b, c)` makes one; and `seq[s]` of a
//! slice `s` slices a built-in sequence as `seq[s.start:s.stop:s.step]` does. A slice's bounds
//! are each an `int` or `None`.

use typy_diagnostics::{Diagnostic, DiagnosticCode, RuleId, Span};
use typy_source::syntax::{Expr, Identifier};

use super::BodyChecker;
use crate::builtin::Builtin;
use crate::typed::{TypedExpr, TypedExprKind};
use crate::types::Type;

/// The names of a slice's bounds, by their positions.
const BOUNDS: [&str; 3] = ["start", "stop", "step"];

impl BodyChecker<'_> {
    /// Returns `slice.start`, `slice.stop` or `slice.step`, each an `int` or `None`.
    pub(super) fn slice_attribute(
        &mut self,
        slice: TypedExpr,
        attribute: &Identifier,
        span: Span,
    ) -> TypedExpr {
        let Some(part) = BOUNDS.iter().position(|bound| *bound == attribute.name) else {
            return self.error(
                Diagnostic::new(
                    DiagnosticCode::error(412),
                    attribute.span,
                    format!(
                        "a slice has no attribute `{}`; it has `start`, `stop` and `step`",
                        attribute.name
                    ),
                )
                .with_rule(RuleId::CLS_23),
                span,
            );
        };
        Self::bound_of(slice, part, span)
    }

    /// Returns the bound at `part` of `slice`.
    fn bound_of(slice: TypedExpr, part: usize, span: Span) -> TypedExpr {
        Self::typed(
            TypedExprKind::SliceBound {
                slice: Box::new(slice),
                part: u8::try_from(part).unwrap_or(0),
            },
            Type::optional(Type::Int),
            span,
        )
    }

    /// Returns the slice `x[lower:upper:step]` passes to a class's method: `slice(lower,
    /// upper, step)` with `None` for each bound not written.
    pub(super) fn slice_object(
        &mut self,
        (lower, upper, step): (Option<&Expr>, Option<&Expr>, Option<&Expr>),
        span: Span,
    ) -> TypedExpr {
        let none = |checker: &mut Self| {
            checker.expression(&Expr::Constant {
                value: typy_source::syntax::Constant::None,
                span,
            })
        };
        let mut values = Vec::with_capacity(3);
        for bound in [lower, upper, step] {
            let value = match bound {
                Some(bound) => self.expression(bound),
                None => none(self),
            };
            self.expect(&value, Type::optional(Type::Int), "a slice bound");
            values.push(value);
        }
        Self::typed(
            TypedExprKind::Builtin {
                builtin: Builtin::Slice,
                arguments: values,
            },
            Type::Slice,
            span,
        )
    }

    /// Returns `sequence[slice]` of a slice object: the sequence sliced by the slice's
    /// bounds.
    pub(super) fn sliced_by_object(
        &mut self,
        sequence: TypedExpr,
        slice: TypedExpr,
        span: Span,
    ) -> TypedExpr {
        let [lower, upper, step] = self.object_bounds(slice, span);
        let ty = sequence.ty.clone();
        Self::typed(
            TypedExprKind::Slice {
                value: Box::new(sequence),
                lower,
                upper,
                step,
            },
            ty,
            span,
        )
    }

    /// Returns the bounds of a slice object, read in order: the slice is held in a hidden
    /// variable as the first is read, and the others read it again.
    pub(super) fn object_bounds(
        &mut self,
        slice: TypedExpr,
        span: Span,
    ) -> [Option<Box<TypedExpr>>; 3] {
        let held = self.hidden_variable("slice", Type::Slice, span);
        let stored = Self::typed(
            TypedExprKind::Named {
                target: held,
                value: Box::new(slice),
            },
            Type::Slice,
            span,
        );
        let read = || Self::typed(TypedExprKind::Local(held), Type::Slice, span);
        [
            Some(Box::new(Self::bound_of(stored, 0, span))),
            Some(Box::new(Self::bound_of(read(), 1, span))),
            Some(Box::new(Self::bound_of(read(), 2, span))),
        ]
    }
}
