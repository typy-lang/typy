//! `yield from` (SMT-11, FUN-07, PEP 380): a generator yields each item an iterable
//! yields, passes on to a generator what resumes it, and gets what that generator returns.

use typy_diagnostics::{Diagnostic, DiagnosticCode, RuleId, Span};
use typy_source::syntax::{Expr, Stmt};

use super::BodyChecker;
use crate::typed::{TypedExprKind, TypedStmt};
use crate::types::Type;

impl BodyChecker<'_> {
    /// Checks `yield from source`, and with `result`, the type an assignment of it receives,
    /// what the source returns: its items must fit what the generator yields, and a
    /// generator it delegates to must receive what the generator is sent.
    pub(super) fn yield_from_statement(
        &mut self,
        source: &Expr,
        result: Option<&mut Type>,
        span: Span,
    ) -> TypedStmt {
        let value = Self::version_tuple(self.expression(source));
        let delegated = match &value.ty {
            Type::Generator { sends, returns, .. } => {
                Some((sends.as_ref().clone(), returns.as_ref().clone()))
            }
            _ => None,
        };
        let value = Self::sequence_of(value);
        let item = self.iterated_item(&value, "`yield from`");
        let source = match &value.ty {
            Type::Iterator(_) | Type::Error => value,
            _ => Self::converted(value, Some(&Type::Iterable(Box::new(item.clone())))),
        };
        let (yields, sends) = self
            .generator
            .as_ref()
            .map_or((Type::Error, Type::Error), |types| {
                (types.yields.clone(), types.sends.clone())
            });
        if !item.has_error() && !item.is_assignable_to(&yields) {
            self.diagnostics.push(
                Diagnostic::new(
                    DiagnosticCode::error(405),
                    source.span,
                    format!(
                        "this yields {}, but the generator yields {yields}",
                        item.with_article()
                    ),
                )
                .with_rule(RuleId::FUN_07),
            );
        }
        if let Some((receives, _)) = &delegated
            && !passes_on(&sends, receives)
        {
            self.diagnostics.push(
                Diagnostic::new(
                    DiagnosticCode::error(405),
                    source.span,
                    format!(
                        "the generator is sent {}, and the one it delegates to receives {}",
                        sends.with_article(),
                        receives.with_article()
                    ),
                )
                .with_rule(RuleId::FUN_07)
                .with_help("annotate both with the same `S` of `Generator[Y, S, R]`"),
            );
        }
        if let Some(result) = result {
            *result = delegated.map_or(Type::NoneType, |(_, returns)| returns);
        }
        let delegated_item = Self::typed(TypedExprKind::DelegatedItem, item, source.span);
        TypedStmt::YieldFrom {
            item: Self::converted(delegated_item, Some(&yields)),
            source,
            received: None,
            span,
        }
    }

    /// Checks `target = yield from source`, also with an annotation: the `yield from`, then
    /// the assignment of what the source returned, `R` of a `Generator[Y, S, R]` and `None`
    /// for any other iterable. An assignment the checks refuse is left out.
    pub(super) fn yield_from_assignment(
        &mut self,
        statement: &Stmt,
        yielded: &Expr,
        span: Span,
    ) -> TypedStmt {
        let Expr::YieldFrom { value, .. } = yielded else {
            unreachable!("the statement assigns a `yield from`")
        };
        let mut result = Type::NoneType;
        let TypedStmt::YieldFrom { source, item, .. } =
            self.yield_from_statement(value, Some(&mut result), span)
        else {
            unreachable!("a `yield from` checks as one")
        };
        self.received = Some(Self::typed(
            TypedExprKind::DelegatedResult,
            result,
            yielded.span(),
        ));
        let received = self.statement(statement).map(Box::new);
        self.received = None;
        TypedStmt::YieldFrom {
            source,
            item,
            received,
            span,
        }
    }
}

/// Returns true when a generator sent values of `sends` may pass them on to one that receives
/// `receives`: the same type, or one an optional of the other, whose values are held alike,
/// or a generator sent nothing but `None`. An optional tuple is held as a pair, so it is not
/// one of a tuple.
fn passes_on(sends: &Type, receives: &Type) -> bool {
    if *sends == Type::NoneType || sends.has_error() || receives.has_error() || sends == receives {
        return true;
    }
    let tuple = |ty: &Type| matches!(ty, Type::Tuple(_));
    (*receives == Type::optional(sends.clone()) && !tuple(sends))
        || (*sends == Type::optional(receives.clone()) && !tuple(receives))
}
