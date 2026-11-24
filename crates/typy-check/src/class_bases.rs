//! The classes of the program each class derives from, which assignability reads, so a class
//! is held where one of its bases is expected, as a `type[Square]` where a `type[Shape]` is
//! (CLS-18, ADR-0084). The table belongs to one check: the scope of `protocol_fits` opens and
//! clears it with the protocols' table, and `ProgramScope` puts it back for lowering.

use std::cell::RefCell;
use std::collections::BTreeMap;

use crate::typed::ClassId;

thread_local! {
    /// The bases of each class of the program, nearest first, while a scope is open.
    static TABLE: RefCell<BTreeMap<ClassId, Vec<ClassId>>> = const { RefCell::new(BTreeMap::new()) };
}

/// Returns true when `class` is `base` or derives from it, in the open scope.
pub(crate) fn derives(class: ClassId, base: ClassId) -> bool {
    class == base
        || TABLE.with(|cell| {
            cell.borrow()
                .get(&class)
                .is_some_and(|bases| bases.contains(&base))
        })
}

/// Notes in the open scope the bases `class` derives from.
pub(crate) fn note(class: ClassId, bases: Vec<ClassId>) {
    TABLE.with(|cell| {
        cell.borrow_mut().insert(class, bases);
    });
}

/// Makes the open scope's table hold `table` alone.
pub(crate) fn reset(table: &BTreeMap<ClassId, Vec<ClassId>>) {
    TABLE.with(|cell| cell.borrow_mut().clone_from(table));
}

/// Returns the open scope's table.
pub(crate) fn table() -> BTreeMap<ClassId, Vec<ClassId>> {
    TABLE.with(|cell| cell.borrow().clone())
}

/// Clears the table, as the scope of a check ends; gives back the table it held.
pub(crate) fn take() -> BTreeMap<ClassId, Vec<ClassId>> {
    TABLE.with(|cell| std::mem::take(&mut *cell.borrow_mut()))
}

#[cfg(test)]
mod tests;
