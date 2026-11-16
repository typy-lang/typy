//! The exception a method of the program raised inside a type object's slot (CLS-22,
//! ADR-0039). A slot returns a `bool` or a hash, so the slot function the compiler writes for
//! a method that may raise keeps the exception here and gives a default. Once one is kept,
//! every slot of a class of the program gives its default without running the method, the
//! runtime's own errors give way to it, a set or a dict adds nothing, and the program raises it
//! after the runtime call. A program has one thread, so one value serves it; each test thread
//! keeps its own, so tests that run at once do not see each other's.

use crate::exception::ExceptionObject;

#[cfg(not(test))]
static PENDING: core::sync::atomic::AtomicPtr<ExceptionObject> =
    core::sync::atomic::AtomicPtr::new(core::ptr::null_mut());

#[cfg(test)]
std::thread_local! {
    static PENDING: core::cell::Cell<*mut ExceptionObject> =
        const { core::cell::Cell::new(core::ptr::null_mut()) };
}

/// Returns the exception kept, or null.
#[cfg(not(test))]
fn get() -> *mut ExceptionObject {
    PENDING.load(core::sync::atomic::Ordering::Relaxed)
}

/// Keeps `exception`, or null, and returns the one kept before.
#[cfg(not(test))]
fn replace(exception: *mut ExceptionObject) -> *mut ExceptionObject {
    PENDING.swap(exception, core::sync::atomic::Ordering::Relaxed)
}

/// Returns the exception kept, or null.
#[cfg(test)]
fn get() -> *mut ExceptionObject {
    PENDING.with(core::cell::Cell::get)
}

/// Keeps `exception`, or null, and returns the one kept before.
#[cfg(test)]
fn replace(exception: *mut ExceptionObject) -> *mut ExceptionObject {
    PENDING.with(|pending| pending.replace(exception))
}

/// Returns true when a slot's method raised an exception the program has not raised yet.
pub(crate) fn pending() -> bool {
    !get().is_null()
}

/// Returns `exception` unless a slot's method raised one first, which is returned instead,
/// with `exception` released, as CPython would have raised the first and never reached the
/// second.
pub(crate) fn or_pending(exception: *mut ExceptionObject) -> *mut ExceptionObject {
    let first = replace(core::ptr::null_mut());
    if first.is_null() {
        return exception;
    }
    // SAFETY: the caller owns `exception`, a live exception it gives up here.
    unsafe { crate::refcount::typy_rt_decrement(exception.cast()) };
    first
}

/// Keeps the exception a slot's method raised, owned by the runtime until the program raises
/// it; a later one, raised by a slot that ran before the program raised the first, is
/// released.
///
/// # Safety
///
/// `exception` is a live exception whose reference the caller gives up.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_slot_failed(exception: *mut ExceptionObject) {
    if pending() {
        // SAFETY: the caller's guarantee.
        unsafe { crate::refcount::typy_rt_decrement(exception.cast()) };
        return;
    }
    replace(exception);
}

/// Returns true when a slot's method raised an exception the program has not raised yet, so
/// a slot gives its default without running its method.
#[unsafe(no_mangle)]
pub extern "C" fn typy_rt_slot_pending() -> bool {
    pending()
}

/// Returns the exception a slot's method raised, which the program raises, and keeps none;
/// null when no slot's method raised one.
#[unsafe(no_mangle)]
pub extern "C" fn typy_rt_slot_raised() -> *mut ExceptionObject {
    replace(core::ptr::null_mut())
}

#[cfg(test)]
mod tests {
    use super::{or_pending, pending, typy_rt_slot_failed, typy_rt_slot_raised};
    use crate::exception::{ExceptionKind, raise};

    #[test]
    fn the_first_exception_a_slot_raises_is_kept_until_the_program_raises_it() {
        assert!(typy_rt_slot_raised().is_null());
        let first = raise(ExceptionKind::ValueError, "first");
        let second = raise(ExceptionKind::TypeError, "second");
        // SAFETY: both exceptions are new and given up here.
        unsafe {
            typy_rt_slot_failed(first);
            typy_rt_slot_failed(second);
        }
        assert!(pending());
        assert_eq!(typy_rt_slot_raised(), first);
        assert!(!pending() && typy_rt_slot_raised().is_null());
        // SAFETY: the program owns the exception it raises.
        unsafe { crate::refcount::typy_rt_decrement(first.cast()) };
    }

    #[test]
    fn a_runtime_error_gives_way_to_the_exception_a_slot_raised() {
        let own = raise(ExceptionKind::KeyError, "own");
        assert_eq!(or_pending(own), own);
        let kept = raise(ExceptionKind::ValueError, "kept");
        // SAFETY: the exception is new and given up here.
        unsafe { typy_rt_slot_failed(kept) };
        let later = raise(ExceptionKind::KeyError, "later");
        assert_eq!(or_pending(later), kept);
        assert!(!pending());
        // SAFETY: the test owns both exceptions it was given.
        unsafe {
            crate::refcount::typy_rt_decrement(own.cast());
            crate::refcount::typy_rt_decrement(kept.cast());
        }
    }
}
