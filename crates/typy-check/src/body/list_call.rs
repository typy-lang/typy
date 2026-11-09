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

    /// Checks `tuple()` or `tuple(source)`. With no argument it is the empty tuple, of the
    /// expected type when that is a `tuple[T, ...]`; with one it is a `tuple[T, ...]` of every
    /// value the source gives (DAT-14).
    pub(super) fn call_tuple(
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
                    "`tuple` takes no keyword arguments",
                )
                .with_rule(RuleId::LIB_02),
                span,
            );
        }
        let list = match (arguments, expected) {
            ([], Some(Type::TupleOf(item))) => {
                let ty = Type::List(item.clone());
                self.list_display(&[], Some(&ty), span)
            }
            ([], _) => {
                return Self::typed(
                    TypedExprKind::Tuple(Vec::new()),
                    Type::Tuple(Vec::new()),
                    span,
                );
            }
            ([_], _) => self.call_list(arguments, keywords, None, span),
            _ => {
                return self.error(
                    Diagnostic::new(
                        DiagnosticCode::error(404),
                        span,
                        format!(
                            "`tuple` takes at most 1 argument, and {} are given",
                            arguments.len()
                        ),
                    )
                    .with_rule(RuleId::LIB_02),
                    span,
                );
            }
        };
        let Some(item) = list.ty.sequence_item().cloned() else {
            return list;
        };
        Self::typed(
            TypedExprKind::TupleOf(Box::new(list)),
            Type::TupleOf(Box::new(item)),
            span,
        )
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
            LoopSource::Items(sequence) if sequence.ty.sequence_item().is_some() => sequence,
            source => Self::typed(
                TypedExprKind::ListOf(Box::new(source)),
                Type::List(Box::new(item)),
                span,
            ),
        }
    }
}
