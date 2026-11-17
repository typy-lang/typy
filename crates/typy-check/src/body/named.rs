//! Checks assignment expressions, `target := value` (EXP-13): the value is checked as an
//! assignment's is, and the expression gives the value itself.

use typy_diagnostics::Span;
use typy_source::syntax::{Expr, Identifier};

use super::BodyChecker;
use crate::typed::{TypedExpr, TypedExprKind};

impl BodyChecker<'_> {
    /// Returns `target := value`. Inside a comprehension the target is the variable of the
    /// enclosing function or module, as `collect_locals` bound it there.
    pub(super) fn named(&mut self, target: &Identifier, value: &Expr, span: Span) -> TypedExpr {
        let name = Expr::Name {
            id: target.name.clone(),
            span: target.span,
        };
        let expected = self.known_type(&name);
        let checked = self.expression_expecting(value, expected.as_ref());
        // The expression gives the value; storing it in a `T | None` variable wraps it there.
        let checked = match checked.kind {
            TypedExprKind::Wrap(inner) => *inner,
            kind => TypedExpr { kind, ..checked },
        };
        let ty = checked.ty.clone();
        let Some(symbol) = self.assign(&name, ty.clone(), &checked) else {
            return Self::invalid_call(span);
        };
        self.named_targets.push(symbol);
        Self::typed(
            TypedExprKind::Named {
                target: symbol,
                value: Box::new(checked),
            },
            ty,
            span,
        )
    }
}
