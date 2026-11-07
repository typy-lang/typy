//! A cell for the runtime's few process-wide values: the output buffer and the stack limit.

use core::cell::UnsafeCell;

/// A value shared by the whole program. Programs are single-threaded up to 1.0 (A-03), so access is never concurrent.
pub(crate) struct ProgramCell<T>(UnsafeCell<T>);

// SAFETY: TyPy programs are single-threaded up to 1.0 (A-03); no two threads touch the value.
unsafe impl<T> Sync for ProgramCell<T> {}

impl<T> ProgramCell<T> {
    pub(crate) const fn new(value: T) -> Self {
        Self(UnsafeCell::new(value))
    }

    /// Runs `action` with the value; the runtime never calls `with` again from inside `action`.
    pub(crate) fn with<R>(&self, action: impl FnOnce(&mut T) -> R) -> R {
        // SAFETY: the program is single-threaded and `with` is never nested, so this is the only reference.
        action(unsafe { &mut *self.0.get() })
    }
}
