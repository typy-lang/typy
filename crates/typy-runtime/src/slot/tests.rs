//! Tests of the exception a method raised inside a type object's slot.

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
    assert!(!pending());
    assert!(typy_rt_slot_raised().is_null());
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
