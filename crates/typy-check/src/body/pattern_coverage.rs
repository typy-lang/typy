//! Finds whether the cases of a `match` cover every value of the subject's type (CST-07,
//! ADR-0033). The type is split into members: each member of a union, `None` of an optional,
//! and `True` and `False` of a `bool`. A case without a guard covers a member when its pattern
//! matches every value of it, so the check never needs the values a pattern tests.

use std::fmt;

use typy_diagnostics::Span;
use typy_source::syntax::{Constant, Expr, MatchCase, Pattern};

use super::BodyChecker;
use super::isinstance::always_instance;
use crate::types::{Type, TypeObjectOf};

/// One part of the subject's type that a case can cover whole.
#[derive(Clone, Debug, PartialEq)]
pub(super) enum Member {
    None,
    True,
    False,
    /// Every value of a type that is not split further.
    Of(Type),
}

impl fmt::Display for Member {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::None => formatter.write_str("None"),
            Self::True => formatter.write_str("True"),
            Self::False => formatter.write_str("False"),
            Self::Of(ty) => write!(formatter, "{ty}"),
        }
    }
}

/// Returns the members of a subject of type `ty`.
pub(super) fn members(ty: &Type) -> Vec<Member> {
    match ty {
        Type::Optional(inner) => std::iter::once(Member::None)
            .chain(members(inner))
            .collect(),
        Type::Union(each) => each.iter().flat_map(members).collect(),
        Type::NoneType => vec![Member::None],
        Type::Bool => vec![Member::True, Member::False],
        other => vec![Member::Of(other.clone())],
    }
}

impl BodyChecker<'_> {
    /// Returns the members of the subject no case without a guard covers.
    pub(super) fn uncovered(&self, ty: &Type, cases: &[MatchCase]) -> Vec<Member> {
        members(ty)
            .into_iter()
            .filter(|member| {
                !cases
                    .iter()
                    .filter(|case| case.guard.is_none())
                    .any(|case| self.covers(&case.pattern, member))
            })
            .collect()
    }

    /// Returns whether `pattern` matches every value of `member`.
    fn covers(&self, pattern: &Pattern, member: &Member) -> bool {
        match pattern {
            Pattern::As { pattern: None, .. } => true,
            Pattern::As {
                pattern: Some(inner),
                ..
            } => self.covers(inner, member),
            Pattern::Or { patterns, .. } => patterns.iter().any(|each| self.covers(each, member)),
            Pattern::Singleton { value, .. } => matches!(
                (value, member),
                (Constant::None, Member::None)
                    | (Constant::Bool(true), Member::True)
                    | (Constant::Bool(false), Member::False)
            ),
            Pattern::Class {
                class,
                patterns,
                keywords,
                span,
            } => {
                patterns.iter().all(irrefutable)
                    && keywords.iter().all(|(_, each)| irrefutable(each))
                    && self
                        .pattern_class(class, *span)
                        .is_some_and(|class| class_covers(&class, member))
            }
            Pattern::Sequence { patterns, .. } => sequence_covers(patterns, member),
            // `{}` and `{**rest}` match every dict.
            Pattern::Mapping { keys, .. } => {
                keys.is_empty() && matches!(member, Member::Of(Type::Dict(..)))
            }
            Pattern::Value { .. } | Pattern::Star { .. } => false,
        }
    }

    /// Returns the class a class pattern names, read without reporting anything: checking
    /// the pattern reported any problem with it already.
    fn pattern_class(&self, class: &Expr, span: Span) -> Option<TypeObjectOf> {
        let Expr::Name { id, .. } = class else {
            return None;
        };
        match self.type_object_named(id, span)?.kind {
            crate::typed::TypedExprKind::TypeObject(of) => Some(*of),
            _ => None,
        }
    }
}

/// Returns whether every value of `member` is an instance of `class`.
fn class_covers(class: &TypeObjectOf, member: &Member) -> bool {
    match member {
        Member::None => false,
        Member::True | Member::False => always_instance(&Type::Bool, class),
        Member::Of(ty) => always_instance(ty, class),
    }
}

/// Returns whether a sequence pattern matches every value of `member`: `[*rest]` matches
/// every list and tuple, and a pattern of captures matches every tuple of its length.
fn sequence_covers(patterns: &[Pattern], member: &Member) -> bool {
    if !patterns.iter().all(irrefutable) {
        return false;
    }
    let starred = patterns
        .iter()
        .any(|each| matches!(each, Pattern::Star { .. }));
    let fixed = patterns.len() - usize::from(starred);
    match member {
        Member::Of(Type::List(_) | Type::TupleOf(_) | Type::Range) => starred && fixed == 0,
        Member::Of(Type::Tuple(items)) => items.len() == fixed || (starred && items.len() >= fixed),
        _ => false,
    }
}

/// Returns whether a pattern matches anything: a capture, `_`, or a starred name.
fn irrefutable(pattern: &Pattern) -> bool {
    matches!(
        pattern,
        Pattern::As { pattern: None, .. } | Pattern::Star { .. }
    )
}
