//! The types of the operators and comparisons of `dict.keys()` and `dict.items()`, which act
//! as sets of their items, and of `dict.values()`, which compares by identity (DAT-16).

use typy_diagnostics::Span;

use super::BodyChecker;
use crate::check::unhashable;
use crate::types::{Type, View};

/// Returns the item type of a view of keys or items.
fn view_item(ty: &Type) -> Option<Type> {
    match ty {
        Type::DictView(View::Keys, key, _) => Some(key.as_ref().clone()),
        Type::DictView(View::Items, key, value) => Some(Type::Tuple(vec![
            key.as_ref().clone(),
            value.as_ref().clone(),
        ])),
        _ => None,
    }
}

/// Returns the item type of a set operand of a view: a view of keys or items, a set, or, for
/// the operators, a list or a `tuple[T, ...]`, which CPython reads as any iterable.
fn operand_item(ty: &Type, operator: bool) -> Option<Type> {
    match ty {
        Type::Set(item) => Some(item.as_ref().clone()),
        Type::List(item) | Type::TupleOf(item) if operator => Some(item.as_ref().clone()),
        _ => view_item(ty),
    }
}

/// Returns whether a type is a view of a dict's keys or items.
pub(super) fn is_set_view(ty: &Type) -> bool {
    view_item(ty).is_some()
}

impl BodyChecker<'_> {
    /// Returns the type of `left op right` for `|`, `&`, `-` or `^` with a view of keys or
    /// items on either side: a set of the items both sides hold, whose type has a hash.
    pub(super) fn view_operation(
        &mut self,
        symbol: &str,
        left: &Type,
        right: &Type,
        span: Span,
    ) -> Type {
        let items = (operand_item(left, true), operand_item(right, true));
        let (Some(item), Some(other)) = items else {
            return self.operator_error(symbol, left, right, span);
        };
        if item != other {
            return self.operator_error(symbol, left, right, span);
        }
        if !item.is_hashable() {
            self.diagnostics.push(unhashable(&item, span));
            return Type::Error;
        }
        Type::Set(Box::new(item))
    }

    /// Returns whether two operands, one of them a view, compare: a view of keys or items with
    /// a set or another such view of the same items, by its items, or two views of values of
    /// one type, by identity, as CPython compares them.
    pub(super) fn view_comparison(left: &Type, right: &Type, equality: bool) -> bool {
        if let (Type::DictView(View::Values, ..), Type::DictView(View::Values, ..)) = (left, right)
        {
            return equality && left == right;
        }
        let items = (operand_item(left, false), operand_item(right, false));
        matches!(items, (Some(item), Some(other)) if item == other)
    }
}
