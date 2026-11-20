//! What a generator's body returns (FUN-07, ADR-0063): `return value` keeps the value in the
//! generator as a one-item tuple, laid out as the descriptor's `result` says, which
//! `StopIteration` then takes as its argument and `yield from` reads as its value.

use super::GeneratorObject;
use crate::Raising;
use crate::exception::{ExceptionKind, ExceptionObject, raise, raise_bare};
use crate::header::{GENERATOR_TYPE_ID, ObjectHeader};

/// Keeps `value`, which `return value` gives with its reference, as the generator's result,
/// in place of one it held. A value that is `None` is not kept, so the generator ends with a
/// `StopIteration` of no arguments, as CPython's does.
///
/// # Safety
///
/// `generator` points to the live generator whose step is running, and `value` is a live
/// item of its return type; one held inline arrives as the address of its bytes, which are
/// copied.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_generator_set_result(generator: *mut GeneratorObject, value: u64) {
    // SAFETY: the caller's guarantee; the descriptor lays out the result.
    unsafe {
        let layout = (*(*generator).descriptor).result;
        if layout.is_null() {
            return;
        }
        let field = crate::tuple::field_type(layout, 0);
        if field.base().is_some() && field.is_none(value) {
            return;
        }
        // A `return` in a `finally` block replaces the value the body returned first.
        release_result(generator);
        let size = usize::try_from((*layout).size).unwrap_or(0);
        let buffer = crate::allocator::allocate(size.max(1));
        crate::tuple::store_field(layout, buffer.expose_provenance() as u64, 0, value);
        (*generator).result = buffer;
    }
}

/// Returns `StopIteration` with the generator's result as its argument, which it takes over,
/// or with none when the body returned `None`.
///
/// # Safety
///
/// `generator` points to a live generator.
pub(super) unsafe fn stop_with_result(generator: *mut GeneratorObject) -> *mut ExceptionObject {
    let exception = raise_bare(ExceptionKind::StopIteration);
    // SAFETY: the caller's guarantee; the result is laid out as the descriptor says.
    unsafe {
        let buffer = core::mem::replace(&mut (*generator).result, core::ptr::null_mut());
        if !buffer.is_null() {
            (*exception).arguments_type = (*(*generator).descriptor).result;
            (*exception).arguments = buffer;
        }
    }
    exception
}

/// Releases the generator's result, if it holds one.
///
/// # Safety
///
/// `generator` points to a live generator.
pub(super) unsafe fn release_result(generator: *mut GeneratorObject) {
    // SAFETY: the caller's guarantee; the result is laid out as the descriptor says.
    unsafe {
        let buffer = core::mem::replace(&mut (*generator).result, core::ptr::null_mut());
        if buffer.is_null() {
            return;
        }
        crate::tuple::release(
            (*(*generator).descriptor).result,
            buffer.expose_provenance() as u64,
        );
        crate::allocator::free(buffer);
    }
}

/// Returns the generator `iterator` is, when it is one.
///
/// # Safety
///
/// `iterator` points to a live object.
pub(super) unsafe fn as_generator(iterator: *mut ObjectHeader) -> Option<*mut GeneratorObject> {
    // SAFETY: the caller's guarantee.
    (unsafe { (*iterator).type_id } == GENERATOR_TYPE_ID).then(|| iterator.cast())
}

/// Returns whether `iterator` is a generator that holds the value its body returned, which
/// `yield from` gives when it ends.
///
/// # Safety
///
/// `iterator` points to a live iterator.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_generator_has_result(iterator: *mut ObjectHeader) -> bool {
    // SAFETY: the caller's guarantee.
    unsafe { as_generator(iterator).is_some_and(|generator| !(*generator).result.is_null()) }
}

/// Returns the value the body of the generator `iterator` returned, borrowed from it, for a
/// `yield from` of a type that cannot be `None`. A generator that ended before the `yield
/// from` read it, or by an exception, holds none, and then this raises `TypeError`.
///
/// # Safety
///
/// `iterator` points to a live iterator.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_generator_result(iterator: *mut ObjectHeader) -> Raising<u64> {
    // SAFETY: the caller's guarantee; the result is laid out as the descriptor says.
    unsafe {
        let Some(generator) = as_generator(iterator) else {
            return Raising::error(0, missing());
        };
        let buffer = (*generator).result;
        if buffer.is_null() {
            return Raising::error(0, missing());
        }
        Raising::ok(crate::tuple::field(
            (*(*generator).descriptor).result,
            buffer.expose_provenance() as u64,
            0,
        ))
    }
}

/// Returns the `TypeError` of a `yield from` whose generator gave no value.
fn missing() -> *mut ExceptionObject {
    raise(
        ExceptionKind::TypeError,
        "the generator of 'yield from' had ended without returning its value",
    )
}
