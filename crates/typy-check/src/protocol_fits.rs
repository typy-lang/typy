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
    /// The table of iterator classes it replaced, put back too (TYP-20).
    iterations: std::collections::BTreeMap<ClassId, crate::iterator_fits::Iteration>,
}

impl FitsScope {
    /// Opens a scope whose table holds `fits`, each a class and a protocol it satisfies, and
    /// whose table of iterator classes is empty.
    pub(crate) fn new(fits: impl IntoIterator<Item = (ClassId, ClassId)>) -> Self {
        let table: BTreeSet<(ClassId, ClassId)> = fits.into_iter().collect();
        let before = FITS.with(|cell| cell.replace(table));
        let iterations = crate::iterator_fits::take();
        Self { before, iterations }
    }
}

/// The classes of a checked program that satisfy its protocols and its iterator classes, put
/// back while a later stage, such as lowering, reads the program's types (ADR-0041, TYP-20).
#[must_use = "the tables are cleared when the scope is dropped"]
pub struct ProgramScope {
    /// The check's scope, whose drop clears the tables again.
    _scope: FitsScope,
}

impl ProgramScope {
    /// Opens a scope with the tables of `program`.
    pub fn open(program: &crate::typed::TypedProgram) -> Self {
        let scope = FitsScope::new(program.fits.0.iter().copied());
        crate::iterator_fits::reset(&program.fits.1);
        Self { _scope: scope }
    }
}

impl Drop for FitsScope {
    fn drop(&mut self) {
        let before = std::mem::take(&mut self.before);
        FITS.with(|cell| cell.replace(before));
        crate::iterator_fits::reset(&self.iterations);
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
