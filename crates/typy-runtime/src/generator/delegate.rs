//! One step of `yield from` (SMT-11, PEP 380): a generator that delegates to an iterator
//! resumes it as the generator itself was resumed, and yields what it yields, until it ends.

use super::{GeneratorObject, Resumption, Step, VALUE, resume};
use crate::Raising;
use crate::exception::{ExceptionKind, ExceptionObject};
use crate::header::ObjectHeader;

/// Takes one step of the iterator `iterator` that `delegator` delegates to, and returns true
/// when it yielded, its item the current one, or false when it ended, its result then kept.
/// The first step is `next()`. A later one passes on how `delegator` was resumed, as CPython
/// does: a value `send` gave is sent, `None` or `next()` is `next()`, and an exception
/// `throw` gave is thrown, except `GeneratorExit`, which closes the iterator first and then
/// leaves. An iterator that is not a generator raises a thrown exception in `delegator`, and
/// a value sent to it raises `AttributeError`.
///
/// # Safety
///
/// `delegator` points to the live generator whose step is running, and `iterator` to a live
/// iterator; a value sent to it is of the type its generator receives.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_generator_delegate(
    delegator: *mut GeneratorObject,
    iterator: *mut ObjectHeader,
    first: bool,
) -> Raising<bool> {
    // SAFETY: the caller's guarantee.
    unsafe {
        let generator = super::result::as_generator(iterator);
        let object = &mut *delegator;
        let how = if first {
            Resumption::Next
        } else {
            let thrown = core::mem::replace(&mut object.thrown, core::ptr::null_mut());
            if !thrown.is_null() {
                return thrown_through(generator, thrown);
            }
            if object.received == VALUE {
                Resumption::Send {
                    sent: object.sent,
                    none: false,
                }
            } else {
                Resumption::Next
            }
        };
        let Some(generator) = generator else {
            if let Resumption::Send { .. } = how {
                return Raising::error(false, no_send(iterator));
            }
            return crate::iterator::typy_rt_iterator_advance(
                iterator.cast(),
                crate::word::inline(0).unwrap_or(1),
            );
        };
        stepped(resume(generator, how))
    }
}

/// Passes `thrown`, owned, to the iterator `yield from` delegates to.
///
/// # Safety
///
/// `generator` is null or points to a live generator, and `thrown` to a live exception.
unsafe fn thrown_through(
    generator: Option<*mut GeneratorObject>,
    thrown: *mut ExceptionObject,
) -> Raising<bool> {
    let Some(generator) = generator else {
        return Raising::error(false, thrown);
    };
    // SAFETY: the caller's guarantee.
    unsafe {
        let class = (*thrown).class();
        if crate::exception::classes::derives_from(class, ExceptionKind::GeneratorExit.class()) {
            let raised = super::typy_rt_generator_close(generator);
            if !raised.is_null() {
                crate::refcount::typy_rt_decrement(thrown.cast());
                return Raising::error(false, raised);
            }
            return Raising::error(false, thrown);
        }
        stepped(resume(generator, Resumption::Throw(thrown)))
    }
}

/// Returns what a step of the delegated generator means to `yield from`.
fn stepped(step: Step) -> Raising<bool> {
    match step {
        Step::Yielded => Raising::ok(true),
        Step::Returned | Step::Finished => Raising::ok(false),
        Step::Raised(exception) => Raising::error(false, exception),
    }
}

/// Returns CPython's `AttributeError` for a value sent through `yield from` to an iterator
/// that is not a generator: `'list_iterator' object has no attribute 'send'`.
///
/// # Safety
///
/// `iterator` points to a live iterator.
unsafe fn no_send(iterator: *mut ObjectHeader) -> *mut ExceptionObject {
    // SAFETY: the caller's guarantee.
    let name = unsafe { crate::type_object::type_of(iterator) }.name();
    let mut text = crate::message::MessageBuffer::new();
    text.push_str("'");
    text.push_str(name);
    text.push_str("' object has no attribute 'send'");
    text.raise(ExceptionKind::AttributeError)
}
