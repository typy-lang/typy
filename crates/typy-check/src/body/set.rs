//! Checks sets: displays, `set()` and `set(source)`, whose items have one type with a hash
//! (DAT-10, TYP-05).

use typy_diagnostics::{Diagnostic, DiagnosticCode, RuleId, Span};
use typy_source::syntax::{Expr, Keyword};

use super::BodyChecker;
use crate::check::unhashable;
use crate::typed::{LoopSource, TypedConstant, TypedExpr, TypedExprKind};
use crate::types::Type;

impl BodyChecker<'_> {
    /// Checks a set display: every item has the type of the first, or the expected item type,
    /// and that type has a hash. A display always has items, since `{}` is an empty dict.
    pub(super) fn set_display(
        &mut self,
        elements: &[Expr],
        expected: Option<&Type>,
        span: Span,
    ) -> TypedExpr {
        let expected_item = match expected {
            Some(Type::Set(item)) => Some(item.as_ref().clone()),
            // The expected type already has its error, so the display gives no second one.
            Some(expected) if expected.has_error() => Some(Type::Error),
            _ => None,
        };
        let items = self.display_items(elements, expected_item.as_ref());
        let item_type = expected_item
            .or_else(|| items.first().map(|first| first.ty.clone()))
            .unwrap_or(Type::Error);
        for item in &items {
            if !item.ty.is_assignable_to(&item_type) {
                self.diagnostics.push(
                    Diagnostic::new(
                        DiagnosticCode::error(409),
                        item.span,
                        format!(
                            "the items of a set have one type; this is {}, and the set holds {item_type}",
                            item.ty.with_article()
                        ),
                    )
                    .with_rule(RuleId::DAT_10),
                );
            }
        }
        if !item_type.is_hashable() {
            return self.error(unhashable(&item_type, span), span);
        }
        Self::typed(
            TypedExprKind::Set(items),
            Type::Set(Box::new(item_type)),
            span,
        )
    }

    /// Checks `set()`, an empty set of the expected type, or `set(source)`, a set of the values
    /// a loop over the source gives.
    pub(super) fn call_set(
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
                    "`set` takes no keyword arguments",
                )
                .with_rule(RuleId::LIB_02),
                span,
            );
        }
        match (arguments, expected) {
            ([], Some(Type::Set(item))) => {
                let ty = Type::Set(item.clone());
                Self::typed(TypedExprKind::Set(Vec::new()), ty, span)
            }
            ([], Some(expected)) if expected.has_error() => Self::invalid_set(span),
            ([], _) => self.error(
                Diagnostic::new(
                    DiagnosticCode::error(410),
                    span,
                    "an empty set needs an annotation that names its item type",
                )
                .with_rule(RuleId::TYP_05)
                .with_help("annotate the variable, such as `items: set[int] = set()`"),
                span,
            ),
            ([source], _) => {
                let (source, item) = self.loop_source(source, "`set`");
                if item.has_error() {
                    return Self::invalid_set(span);
                }
                if !item.is_hashable() {
                    return self.error(unhashable(&item, span), span);
                }
                Self::typed(
                    TypedExprKind::SetOf(Box::new(source)),
                    Type::Set(Box::new(item)),
                    span,
                )
            }
            _ => self.error(
                Diagnostic::new(
                    DiagnosticCode::error(404),
                    span,
                    format!(
                        "`set` takes at most 1 argument, and {} are given",
                        arguments.len()
                    ),
                )
                .with_rule(RuleId::LIB_02),
                span,
            ),
        }
    }

    /// Checks an argument of a set method that takes any iterable of `item` (DAT-15). A set, a
    /// dict, a list or a `tuple[T, ...]` is passed as it is; any other source becomes a list of the
    /// values it gives, in order, which the runtime reads as CPython reads the iterable. A
    /// display, `[]` or `set()` takes the type the method wants.
    pub(super) fn items_argument(&mut self, argument: &Expr, item: &Type, what: &str) -> TypedExpr {
        let span = argument.span();
        let expected = match argument {
            Expr::Set { .. } => Some(Type::Set(Box::new(item.clone()))),
            Expr::List { .. } => Some(Type::List(Box::new(item.clone()))),
            _ if self.is_empty_set_call(argument) => Some(Type::Set(Box::new(item.clone()))),
            _ => None,
        };
        if let Some(expected) = expected {
            let value = self.expression_expecting(argument, Some(&expected));
            self.expect(&value, expected, what);
            return value;
        }
        let (source, given) = self.loop_source(argument, what);
        if given.has_error() {
            return Self::invalid_set(span);
        }
        if !given.is_assignable_to(item) {
            return self.error(
                Diagnostic::new(
                    DiagnosticCode::error(404),
                    span,
                    format!("{what} must give items of type {item}, and these items are {given}"),
                )
                .with_rule(RuleId::DAT_15),
                span,
            );
        }
        match source {
            LoopSource::Items(sequence)
                if matches!(sequence.ty, Type::Set(_) | Type::Dict(..))
                    || sequence.ty.sequence_item().is_some() =>
            {
                sequence
            }
            source => Self::typed(
                TypedExprKind::ListOf(Box::new(source)),
                Type::List(Box::new(item.clone())),
                span,
            ),
        }
    }

    fn invalid_set(span: Span) -> TypedExpr {
        Self::typed(
            TypedExprKind::Constant(TypedConstant::None),
            Type::Error,
            span,
        )
    }
}
