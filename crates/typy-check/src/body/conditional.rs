//! Checks conditional expressions, `body if test else orelse` (EXP-11): both branches of one
//! type, each checked with what the test shows, and given the `Callable` the context expects
//! so a `lambda` branch takes its types from it (EXP-14).

use typy_diagnostics::{Diagnostic, DiagnosticCode, RuleId, Span};
use typy_source::syntax::Expr;

use super::BodyChecker;
use super::flow::test_facts;
use crate::typed::{TypedExpr, TypedExprKind};
use crate::types::Type;

impl BodyChecker<'_> {
    /// Returns the checked conditional expression of `test`, `body` and `orelse`, whose
    /// branches are checked for the type `expected` when the context expects one.
    pub(super) fn conditional(
        &mut self,
        (test, body, orelse): (&Expr, &Expr, &Expr),
        expected: Option<&Type>,
        span: Span,
    ) -> TypedExpr {
        let test = self.truth_test(test);
        let (holds, fails) = test_facts(&test);
        let outer = self.assigned.clone();
        self.add_facts(holds);
        let body = self.branch(body, expected);
        self.assigned.clone_from(&outer);
        self.add_facts(fails);
        let orelse = self.branch(orelse, expected);
        self.assigned = outer;
        // A branch of `None` and one of `T` give `T | None`, as Python's typing joins
        // them (TYP-06).
        let joined = match (&body.ty, &orelse.ty) {
            (Type::NoneType, other) | (other, Type::NoneType)
                if !matches!(other, Type::NoneType) =>
            {
                Some(Type::optional(other.clone()))
            }
            (Type::Optional(_), _) => Some(body.ty.clone()),
            (_, Type::Optional(_)) => Some(orelse.ty.clone()),
            _ => None,
        };
        let (body, orelse) = match &joined {
            Some(joined) => (
                Self::converted(body, Some(joined)),
                Self::converted(orelse, Some(joined)),
            ),
            None => (body, orelse),
        };
        if !orelse.ty.is_assignable_to(&body.ty) {
            return self.error(
                Diagnostic::new(
                    DiagnosticCode::error(305),
                    orelse.span,
                    format!(
                        "both branches must have one type; they are {} and {}",
                        body.ty, orelse.ty
                    ),
                )
                .with_rule(RuleId::EXP_11),
                span,
            );
        }
        let ty = body.ty.clone();
        Self::typed(
            TypedExprKind::IfExp {
                test: Box::new(test),
                body: Box::new(body),
                orelse: Box::new(orelse),
            },
            ty,
            span,
        )
    }

    /// Returns one branch, checked for the type `expected` when the context expects one.
    fn branch(&mut self, branch: &Expr, expected: Option<&Type>) -> TypedExpr {
        match expected {
            Some(expected) => self.expression_expecting(branch, Some(expected)),
            None => self.expression(branch),
        }
    }
}
