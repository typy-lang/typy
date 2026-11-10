//! The exception an `except` clause is handling, which an exception raised meanwhile takes as
//! its context (EXC-04, ADR-0013). A program has one thread, so one value serves it; each
//! test thread keeps its own, so tests that run at once do not see each other's.

use super::ExceptionObject;

#[cfg(not(test))]
static HANDLED: core::sync::atomic::AtomicPtr<ExceptionObject> =
    core::sync::atomic::AtomicPtr::new(core::ptr::null_mut());

#[cfg(test)]
std::thread_local! {
    static HANDLED: core::cell::Cell<*mut ExceptionObject> =
        const { core::cell::Cell::new(core::ptr::null_mut()) };
}

/// Returns the exception being handled, or null.
#[cfg(not(test))]
pub(super) fn get() -> *mut ExceptionObject {
    HANDLED.load(core::sync::atomic::Ordering::Relaxed)
}

/// Makes `exception` the one being handled, and returns the one handled before.
#[cfg(not(test))]
pub(super) fn replace(exception: *mut ExceptionObject) -> *mut ExceptionObject {
    HANDLED.swap(exception, core::sync::atomic::Ordering::Relaxed)
}

/// Returns the exception being handled, or null.
#[cfg(test)]
pub(super) fn get() -> *mut ExceptionObject {
    HANDLED.with(core::cell::Cell::get)
}

/// Makes `exception` the one being handled, and returns the one handled before.
#[cfg(test)]
pub(super) fn replace(exception: *mut ExceptionObject) -> *mut ExceptionObject {
    HANDLED.with(|handled| handled.replace(exception))
}
