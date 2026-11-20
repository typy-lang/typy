//! The generators freed while suspended at a `yield`, which wait to be closed (FUN-11,
//! ADR-0064). A release never runs code of the program: freeing such a generator puts it at
//! the end of this queue, and compiled code closes the queue's generators as soon as the call
//! that dropped them returns. A program has one thread, so one queue serves it; each test
//! thread keeps its own, so tests that run at once do not see each other's.

use super::GeneratorObject;

/// The first generator of the queue, null when it is empty, which compiled code reads after
/// each call that may release an object.
#[cfg(not(test))]
#[unsafe(export_name = "typy_rt_generator_dropped")]
static FIRST: core::sync::atomic::AtomicPtr<GeneratorObject> =
    core::sync::atomic::AtomicPtr::new(core::ptr::null_mut());

/// The last generator of the queue, null when it is empty.
#[cfg(not(test))]
static LAST: core::sync::atomic::AtomicPtr<GeneratorObject> =
    core::sync::atomic::AtomicPtr::new(core::ptr::null_mut());

#[cfg(test)]
std::thread_local! {
    static FIRST: core::cell::Cell<*mut GeneratorObject> =
        const { core::cell::Cell::new(core::ptr::null_mut()) };
    static LAST: core::cell::Cell<*mut GeneratorObject> =
        const { core::cell::Cell::new(core::ptr::null_mut()) };
}

#[cfg(not(test))]
fn first() -> *mut GeneratorObject {
    FIRST.load(core::sync::atomic::Ordering::Relaxed)
}

#[cfg(not(test))]
fn set_ends(first: *mut GeneratorObject, last: *mut GeneratorObject) {
    FIRST.store(first, core::sync::atomic::Ordering::Relaxed);
    LAST.store(last, core::sync::atomic::Ordering::Relaxed);
}

#[cfg(not(test))]
fn last() -> *mut GeneratorObject {
    LAST.load(core::sync::atomic::Ordering::Relaxed)
}

#[cfg(test)]
fn first() -> *mut GeneratorObject {
    FIRST.with(core::cell::Cell::get)
}

#[cfg(test)]
fn set_ends(first: *mut GeneratorObject, last: *mut GeneratorObject) {
    FIRST.with(|each| each.set(first));
    LAST.with(|each| each.set(last));
}

#[cfg(test)]
fn last() -> *mut GeneratorObject {
    LAST.with(core::cell::Cell::get)
}

/// Puts `generator` at the end of the queue.
///
/// # Safety
///
/// `generator` is a suspended generator whose last reference is gone.
pub(super) unsafe fn push(generator: *mut GeneratorObject) {
    // SAFETY: the caller's guarantee; a generator in the queue is alive until it is closed.
    unsafe {
        (*generator).dropped = core::ptr::null_mut();
        let previous = last();
        if previous.is_null() {
            set_ends(generator, generator);
        } else {
            (*previous).dropped = generator;
            set_ends(first(), generator);
        }
    }
}

/// Returns true when the queue is empty.
pub(super) fn is_empty() -> bool {
    first().is_null()
}

/// Takes every generator out of the queue, leaving it empty, and returns the first, which
/// links to the others in order through `dropped`; null when the queue is empty.
pub(super) fn take() -> *mut GeneratorObject {
    let generator = first();
    set_ends(core::ptr::null_mut(), core::ptr::null_mut());
    generator
}
