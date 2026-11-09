//! Checks `list(...)`, and the iterables the builtins of one iterable take (LIB-02).

use typy_diagnostics::{Diagnostic, DiagnosticCode, RuleId, Span};
use typy_source::syntax::{Expr, Keyword};

use super::BodyChecker;
use crate::builtin::Builtin;
use crate::typed::{LoopSource, TypedConstant, TypedExpr, TypedExprKind};
use crate::types::Type;

impl BodyChecker<'_> {
    /// Checks `list()` or `list(source)`. With no argument the list has the expected type, as an
    /// empty display does (TYP-05); with one it holds every value the source gives: the items
    /// of a list, the characters of a `str`, or what `enumerate`, `zip` or `reversed` give.
    pub(super) fn call_list(
        &mut self,
        arguments: &[Expr],
        keywords: &[Keyword],
        expected: Option<&Type>,
        span: Span,
    ) -> TypedExpr {
        if let Some(keyword) = keywords.first() {
            return self.error(
                Diagnostic::new(
                    DiagnosticCode::error(404),
                    keyword.span,
                    "`list` takes no keyword arguments",
                )
                .with_rule(RuleId::LIB_02),
                span,
            );
        }
        match arguments {
            [] => self.list_display(&[], expected, span),
            [source] => {
                let (source, item) = self.loop_source(source, "`list`");
                if item.has_error() {
                    return Self::typed(
                        TypedExprKind::Constant(TypedConstant::None),
                        Type::Error,
                        span,
                    );
                }
                Self::typed(
                    TypedExprKind::ListOf(Box::new(source)),
                    Type::List(Box::new(item)),
                    span,
                )
            }
            _ => self.error(
                Diagnostic::new(
                    DiagnosticCode::error(404),
                    span,
                    format!(
                        "`list` takes at most 1 argument, and {} are given",
                        arguments.len()
                    ),
                )
                .with_rule(RuleId::LIB_02),
                span,
            ),
        }
    }

    /// Checks the iterable argument of `builtin` and returns it as a list: a list as it is, and
    /// any other source, a `str` or `enumerate`, `zip` or `reversed`, as a new list of its
    /// values, which the builtin then reads.
    pub(super) fn iterable_argument(&mut self, argument: &Expr, builtin: Builtin) -> TypedExpr {
        let span = argument.span();
        let (source, item) = self.loop_source(argument, &format!("`{}`", builtin.name()));
        if item.has_error() {
            return Self::typed(
                TypedExprKind::Constant(TypedConstant::None),
                Type::Error,
                span,
            );
        }
        match source {
            LoopSource::Items(sequence) if matches!(sequence.ty, Type::List(_)) => sequence,
            source => Self::typed(
                TypedExprKind::ListOf(Box::new(source)),
                Type::List(Box::new(item)),
                span,
            ),
        }
    }
}
