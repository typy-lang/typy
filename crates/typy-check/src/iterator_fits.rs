//! The classes of the program that are iterators or iterables, which assignability takes into
//! account, so an instance is held and passed where an `Iterator[T]` or an `Iterable[T]` is
//! expected, as `typing` reads the protocols (TYP-20, ADR-0065). The table belongs to one
//! check: the scope of `protocol_fits` opens and clears it with the protocols' table.

use std::cell::RefCell;
use std::collections::BTreeMap;

use crate::typed::ClassId;
use crate::types::Type;

/// What a class's instances are, beyond the class.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Iteration {
    /// `__next__` returns the type, and `__iter__` returns the instance: an `Iterator[T]`,
    /// and so an `Iterable[T]` too.
    Iterator(Type),
    /// `__iter__` returns an `Iterator[T]`: an `Iterable[T]`.
    Iterable(Type),
}

impl Iteration {
    /// Returns the type of each item.
    pub(crate) const fn item(&self) -> &Type {
        match self {
            Self::Iterator(item) | Self::Iterable(item) => item,
        }
    }
}

thread_local! {
    /// What each class's instances are, while a scope is open.
    static TABLE: RefCell<BTreeMap<ClassId, Iteration>> = const { RefCell::new(BTreeMap::new()) };
}

/// Returns what the instances of `class` are, in the open scope.
pub(crate) fn iteration(class: ClassId) -> Option<Iteration> {
    TABLE.with(|cell| cell.borrow().get(&class).cloned())
}

/// Notes in the open scope what the instances of `class` are.
pub(crate) fn note(class: ClassId, iteration: Iteration) {
    TABLE.with(|cell| {
        cell.borrow_mut().insert(class, iteration);
    });
}

/// Makes the open scope's table hold `table` alone, as `protocol_fits::reset` does.
pub(crate) fn reset(table: &BTreeMap<ClassId, Iteration>) {
    TABLE.with(|cell| cell.borrow_mut().clone_from(table));
}

/// Clears the table, as the scope of a check ends; gives back the table it held.
pub(crate) fn take() -> BTreeMap<ClassId, Iteration> {
    TABLE.with(|cell| std::mem::take(&mut *cell.borrow_mut()))
}

#[cfg(test)]
mod tests;
