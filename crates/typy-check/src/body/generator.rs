//! Generator functions (SMT-11, FUN-07, ADR-0063): a function whose body has a `yield`
//! makes a generator when it is called, and its annotation, `Generator[Y, S, R]`,
//! `Iterator[Y]` or `Iterable[Y]`, gives the type each `yield` gives, what `send` passes
//! and what its `return` gives.

use typy_diagnostics::{Diagnostic, DiagnosticCode, RuleId, Span};
use typy_source::syntax::{Expr, Stmt};

use super::BodyChecker;
use crate::check::Definition;
use crate::typed::{GeneratorTypes, TypedConstant, TypedExpr, TypedExprKind, TypedStmt};
use crate::types::Type;

impl BodyChecker<'_> {
    /// Returns the types of the generator function `definition`, whose body is `body`, or
    /// `None` when the body has no `yield`. Any annotation other than a generator, an
    /// iterator or an iterable is E0405.
    pub(super) fn generator_types(
        &mut self,
        definition: &Definition<'_>,
        body: &[Stmt],
    ) -> Option<GeneratorTypes> {
        if !body.iter().any(has_yield) {
            return None;
        }
        let none = || Box::new(Type::NoneType);
        let (yields, sends, returns) = match &definition.signature.returns {
            Type::Generator {
                yields,
                sends,
                returns,
            } => (yields.clone(), sends.clone(), returns.clone()),
            Type::Iterator(yields) | Type::Iterable(yields) => (yields.clone(), none(), none()),
            Type::Error => (Box::new(Type::Error), none(), none()),
            other => {
                self.diagnostics.push(
                    Diagnostic::new(
                        DiagnosticCode::error(405),
                        definition.span,
                        format!(
                            "`{}` has a `yield`, so its call makes a generator, and it is \
                             annotated as returning {}",
                            definition.name,
                            other.with_article()
                        ),
                    )
                    .with_rule(RuleId::FUN_07)
                    .with_help(
                        "annotate it `-> Iterator[T]`, `-> Iterable[T]` or \
                         `-> Generator[T, S, R]`, of `collections.abc` or `typing`",
                    ),
                );
                (Box::new(Type::Error), none(), none())
            }
        };
        Some(GeneratorTypes {
            yields: *yields,
            sends: *sends,
            returns: *returns,
        })
    }

    /// Checks `yield value`, or a bare `yield`, which gives `None`, against the type the
    /// generator yields.
    pub(super) fn yield_statement(&mut self, value: Option<&Expr>, span: Span) -> TypedStmt {
        let yields = self
            .generator
            .as_ref()
            .map_or(Type::Error, |types| types.yields.clone());
        let value = match value {
            Some(value) => self.expression_expecting(value, Some(&yields)),
            None => Self::typed(
                TypedExprKind::Constant(TypedConstant::None),
                Type::NoneType,
                span,
            ),
        };
        let value = Self::converted(value, Some(&yields));
        if !value.ty.is_assignable_to(&yields) {
            self.diagnostics.push(
                Diagnostic::new(
                    DiagnosticCode::error(405),
                    value.span,
                    format!(
                        "this yields {}, but the generator yields {yields}",
                        value.ty.with_article()
                    ),
                )
                .with_rule(RuleId::FUN_07),
            );
        }
        TypedStmt::Yield {
            value,
            received: None,
            span,
        }
    }

    /// Checks `target = yield value`, also with an annotation: the `yield`, then the
    /// assignment of what the next step receives, `S | None` of a `Generator[Y, S, R]`, since
    /// `next` and a `for` loop resume it with `None` (SMT-11, ADR-0067). An assignment the
    /// checks refuse is left out.
    pub(super) fn yield_assignment(
        &mut self,
        statement: &Stmt,
        yielded: &Expr,
        span: Span,
    ) -> TypedStmt {
        let Expr::Yield { value, .. } = yielded else {
            unreachable!("the statement assigns a `yield`")
        };
        let TypedStmt::Yield { value, .. } = self.yield_statement(value.as_deref(), span) else {
            unreachable!("a `yield` checks as one")
        };
        let sends = self
            .generator
            .as_ref()
            .map_or(Type::Error, |types| types.sends.clone());
        self.received = Some(Self::typed(
            TypedExprKind::Received,
            Type::optional(sends),
            yielded.span(),
        ));
        let received = self.statement(statement).map(Box::new);
        self.received = None;
        TypedStmt::Yield {
            value,
            received,
            span,
        }
    }

    /// Returns what the `yield` of the assignment being checked receives (SMT-11).
    pub(super) fn received_value(&mut self, span: Span) -> TypedExpr {
        self.received.take().unwrap_or_else(|| {
            // The subset check refuses a `yield` anywhere else (FUN-12).
            Self::typed(
                TypedExprKind::Constant(TypedConstant::None),
                Type::Error,
                span,
            )
        })
    }
}

/// Returns true when the statement has a `yield` or a `yield from` of its function: in it or
/// in a block it holds, outside the functions and classes it defines.
fn has_yield(statement: &Stmt) -> bool {
    let mut found = false;
    crate::check::visit_statement(statement, &mut |expression| {
        found |= matches!(expression, Expr::Yield { .. } | Expr::YieldFrom { .. });
    });
    found
}
