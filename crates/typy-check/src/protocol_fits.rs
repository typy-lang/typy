//! The classes that satisfy each protocol, which the test of one class deriving from another
//! takes into account, so an instance converts to a protocol it satisfies as it converts to a
//! base (GEN-10, ADR-0041). The table belongs to one check: a `FitsScope` sets it and clears it
//! when it ends, also after a failure.

use std::cell::RefCell;
use std::collections::BTreeSet;

use crate::typed::ClassId;

thread_local! {
    /// Each pair of a class and a protocol it satisfies, while a scope is open.
    static FITS: RefCell<BTreeSet<(ClassId, ClassId)>> = const { RefCell::new(BTreeSet::new()) };
}

/// The table of fits of one check, cleared when the scope ends.
#[must_use = "the table is cleared when the scope is dropped"]
pub(crate) struct FitsScope {
    /// The table the scope replaced, put back when it ends.
    before: BTreeSet<(ClassId, ClassId)>,
}

impl FitsScope {
    /// Opens a scope whose table holds `fits`, each a class and a protocol it satisfies.
    pub(crate) fn new(fits: impl IntoIterator<Item = (ClassId, ClassId)>) -> Self {
        let table: BTreeSet<(ClassId, ClassId)> = fits.into_iter().collect();
        let before = FITS.with(|cell| cell.replace(table));
        Self { before }
    }
}

impl Drop for FitsScope {
    fn drop(&mut self) {
        let before = std::mem::take(&mut self.before);
        FITS.with(|cell| cell.replace(before));
    }
}

/// Returns true when `class` satisfies `protocol` in the open scope.
pub(crate) fn fits(class: ClassId, protocol: ClassId) -> bool {
    FITS.with(|cell| cell.borrow().contains(&(class, protocol)))
}

/// Notes in the open scope that `class` satisfies `protocol`.
pub(crate) fn note(class: ClassId, protocol: ClassId) {
    FITS.with(|cell| {
        cell.borrow_mut().insert((class, protocol));
    });
}

/// Makes the open scope's table hold `fits` alone: a check that starts again, or a first look
/// at a module body, whose copies' IDs are given back, starts from them.
pub(crate) fn reset(fits: &BTreeSet<(ClassId, ClassId)>) {
    FITS.with(|cell| cell.borrow_mut().clone_from(fits));
}

#[cfg(test)]
mod tests {
    use super::{FitsScope, fits, note};
    use crate::typed::ClassId;

    #[test]
    fn a_scope_holds_its_fits_and_puts_back_the_table_it_replaced() {
        let outer = FitsScope::new([(ClassId(1), ClassId(2))]);
        {
            let _inner = FitsScope::new([]);
            note(ClassId(3), ClassId(2));
            assert!(fits(ClassId(3), ClassId(2)));
            assert!(!fits(ClassId(1), ClassId(2)));
        }
        assert!(fits(ClassId(1), ClassId(2)));
        assert!(!fits(ClassId(3), ClassId(2)));
        drop(outer);
        assert!(!fits(ClassId(1), ClassId(2)));
    }
}
