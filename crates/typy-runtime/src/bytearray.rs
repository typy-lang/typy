//! `bytearray`: a mutable sequence of bytes, held in a `bytes` buffer it owns with room to
//! grow, so every function of `bytes` reads it in place (LIB-28, ADR-0108).
//!
//! The buffer has the type ID of `bytes` and the bytearray's length as its own, and its memory
//! holds `capacity` bytes and a zero after the last one. It is never hashed and never given to
//! the program, so no other reference to it exists.

mod methods;

use crate::Raising;
use crate::allocator::{allocate, free};
use crate::bytes::{from_slice, new_bytes};
use crate::exception::{ExceptionKind, ExceptionObject, raise};
use crate::header::{BYTEARRAY_TYPE_ID, IMMORTAL, ObjectHeader};
use crate::string::{StrObject, latin1_data, length, reserve_latin1};
use crate::word::{inline, inline_value, is_inline};

pub(crate) use methods::*;

/// A `bytearray`.
#[repr(C)]
pub struct BytearrayObject {
    /// The object header.
    pub header: ObjectHeader,
    /// The bytes, a `bytes` object of the bytearray's length.
    pub buffer: *mut StrObject,
    /// The number of bytes the buffer has room for.
    pub capacity: usize,
}

/// The message of `ValueError` for an item outside 0 to 255.
const BYTE_RANGE: &str = "byte must be in range(0, 256)";

/// Returns a new bytearray that owns `buffer`, which has room for `capacity` bytes.
fn wrap(buffer: *mut StrObject, capacity: usize) -> *mut BytearrayObject {
    #[allow(
        clippy::cast_ptr_alignment,
        reason = "malloc returns memory aligned for every type"
    )]
    let object = allocate(core::mem::size_of::<BytearrayObject>()).cast::<BytearrayObject>();
    // SAFETY: the memory is new and as large as the object.
    unsafe {
        object.write(BytearrayObject {
            header: ObjectHeader::new(BYTEARRAY_TYPE_ID),
            buffer,
            capacity,
        });
    }
    object
}

/// Returns a new bytearray of `data`.
pub(crate) fn from_data(data: &[u8]) -> *mut BytearrayObject {
    wrap(from_slice(data), data.len())
}

/// Returns the bytes of `array`, borrowed from its buffer.
///
/// # Safety
///
/// `array` points to a live bytearray, which outlives the slice and does not change while
/// the slice is read.
pub(crate) unsafe fn bytes_of<'a>(array: *const BytearrayObject) -> &'a [u8] {
    // SAFETY: the buffer holds `length` bytes.
    unsafe { core::slice::from_raw_parts(latin1_data((*array).buffer), length((*array).buffer)) }
}

/// Returns the bytes of a `bytes` or of a bytearray's buffer.
///
/// # Safety
///
/// `data` points to a live `bytes`, which outlives the slice.
pub(crate) unsafe fn data_of<'a>(data: *const StrObject) -> &'a [u8] {
    // SAFETY: a `bytes` holds `length` bytes, one per code point.
    unsafe { core::slice::from_raw_parts(latin1_data(data), length(data)) }
}

/// Makes room for at least `needed` bytes, doubling the capacity, so appending one byte at a
/// time takes constant time on average.
///
/// # Safety
///
/// `array` points to a live bytearray.
unsafe fn reserve(array: *mut BytearrayObject, needed: usize) {
    // SAFETY: the bytearray owns its buffer, which no one else refers to.
    unsafe {
        if needed <= (*array).capacity {
            return;
        }
        let capacity = needed.max((*array).capacity.saturating_mul(2)).max(8);
        (*array).buffer = reserve_latin1((*array).buffer, capacity);
        (*array).capacity = capacity;
    }
}

/// Gives `array` the length `size`, within its capacity, and the zero after its last byte.
///
/// # Safety
///
/// `array` points to a live bytearray with room for `size` bytes.
unsafe fn set_length(array: *mut BytearrayObject, size: usize) {
    // SAFETY: the buffer has room for `capacity` bytes and the zero after them.
    unsafe {
        (*(*array).buffer).length = size as u64;
        *latin1_data((*array).buffer).add(size) = 0;
    }
}

/// Returns the bytes of `array` to write, all `capacity` of them.
///
/// # Safety
///
/// `array` points to a live bytearray, which nothing else changes while the slice is used.
unsafe fn room<'a>(array: *mut BytearrayObject) -> &'a mut [u8] {
    // SAFETY: the buffer has room for `capacity` bytes.
    unsafe { core::slice::from_raw_parts_mut(latin1_data((*array).buffer), (*array).capacity) }
}

/// Returns the byte an `int` word holds, or `ValueError` for one outside 0 to 255.
fn byte(word: i64) -> Result<u8, *mut ExceptionObject> {
    if is_inline(word)
        && let Ok(value) = u8::try_from(inline_value(word))
    {
        return Ok(value);
    }
    Err(raise(ExceptionKind::ValueError, BYTE_RANGE))
}

/// Returns the position an `int` word names in `array`, counted from the end when negative,
/// or `IndexError` with `message` for one outside it.
///
/// # Safety
///
/// `array` points to a live bytearray.
unsafe fn position(
    array: *const BytearrayObject,
    index: i64,
    message: &str,
) -> Result<usize, *mut ExceptionObject> {
    if !is_inline(index) {
        return Err(raise(
            ExceptionKind::IndexError,
            "cannot fit 'int' into an index-sized integer",
        ));
    }
    // SAFETY: the caller passes a live bytearray.
    let size = i64::try_from(unsafe { length((*array).buffer) }).unwrap_or(i64::MAX);
    let index = inline_value(index);
    let from_start = if index < 0 { index + size } else { index };
    if (0..size).contains(&from_start) {
        Ok(usize::try_from(from_start).unwrap_or(0))
    } else {
        Err(raise(ExceptionKind::IndexError, message))
    }
}

/// Frees a bytearray whose last reference is gone, with its buffer.
///
/// # Safety
///
/// `array` points to a bytearray that nothing refers to any more.
pub(crate) unsafe fn destroy(array: *mut BytearrayObject) {
    // SAFETY: the bytearray alone owns its buffer.
    unsafe {
        free((*array).buffer.cast());
        free(array.cast());
    }
}

/// Returns `bytearray(count)`: `count` zero bytes; a negative count raises `ValueError`.
///
/// # Safety
///
/// `count` is an `int` word.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_bytearray_new(count: i64) -> Raising<*mut BytearrayObject> {
    let count = crate::slice::clamped(count);
    let Ok(count) = usize::try_from(count) else {
        return Raising::error(
            core::ptr::null_mut(),
            raise(ExceptionKind::ValueError, "negative count"),
        );
    };
    let buffer = new_bytes(count);
    let array = wrap(buffer, count);
    // SAFETY: the bytearray is new and has room for `count` bytes.
    unsafe {
        room(array).fill(0);
        set_length(array, count);
    }
    Raising::ok(array)
}

/// Returns `bytearray(data)` of a `bytes` or of a bytearray's buffer: a copy of its bytes.
///
/// # Safety
///
/// `data` points to a live `bytes`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_bytearray_from_bytes(
    data: *const StrObject,
) -> *mut BytearrayObject {
    // SAFETY: the caller's guarantee.
    from_data(unsafe { data_of(data) })
}

/// Returns `bytearray(values)` of a list of `int` items; one outside 0 to 255 raises
/// `ValueError`.
///
/// # Safety
///
/// `values` points to a live list of `int` items.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_bytearray_from_list(
    values: *const crate::ListObject,
) -> Raising<*mut BytearrayObject> {
    // SAFETY: the caller passes a live list of `int` words.
    unsafe {
        let count = (*values).length;
        let array = wrap(new_bytes(count), count);
        for index in 0..count {
            match byte((*values).item(index).cast_signed()) {
                Ok(value) => room(array)[index] = value,
                Err(error) => {
                    destroy(array);
                    return Raising::error(core::ptr::null_mut(), error);
                }
            }
        }
        set_length(array, count);
        Raising::ok(array)
    }
}

/// Returns a new bytearray of `data`, a `bytes` whose reference the caller gives up: it becomes
/// the buffer when the caller held its only reference, and is copied otherwise, so no two
/// bytearrays share a buffer (ADR-0108).
///
/// # Safety
///
/// `data` points to a live `bytes`, and the caller owns a reference to it.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_bytearray_adopt(data: *mut StrObject) -> *mut BytearrayObject {
    // SAFETY: the caller's guarantee.
    unsafe {
        let count = (*data).header.reference_count;
        if count == 1 && count != IMMORTAL {
            return wrap(data, length(data));
        }
        let array = from_data(data_of(data));
        crate::refcount::typy_rt_decrement(data.cast());
        array
    }
}

/// Returns the buffer of `array`, borrowed: a `bytes` that the functions of `bytes` read.
///
/// # Safety
///
/// `array` points to a live bytearray.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_bytearray_buffer(
    array: *const BytearrayObject,
) -> *const StrObject {
    // SAFETY: the caller's guarantee.
    unsafe { (*array).buffer }
}

/// Returns `bytes(array)`: a new `bytes` of the bytearray's bytes as they are now.
///
/// # Safety
///
/// `array` points to a live bytearray.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_bytearray_to_bytes(
    array: *const BytearrayObject,
) -> *mut StrObject {
    // SAFETY: the caller's guarantee.
    from_slice(unsafe { bytes_of(array) })
}

/// Returns `len(array)`, as an `int` word.
///
/// # Safety
///
/// `array` points to a live bytearray.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_bytearray_length(array: *const BytearrayObject) -> i64 {
    // SAFETY: the caller's guarantee.
    let size = unsafe { bytes_of(array).len() };
    inline(i64::try_from(size).unwrap_or(i64::MAX)).unwrap_or(1)
}

/// Returns `array[index]`, as an `int` word; a position outside raises `IndexError`.
///
/// # Safety
///
/// `array` points to a live bytearray, and `index` is an `int` word.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_bytearray_get(
    array: *const BytearrayObject,
    index: i64,
) -> Raising<i64> {
    // SAFETY: the caller's guarantee; `position` keeps the index inside.
    unsafe {
        match position(array, index, "bytearray index out of range") {
            Ok(at) => Raising::ok(inline(i64::from(bytes_of(array)[at])).unwrap_or(1)),
            Err(error) => Raising::error(1, error),
        }
    }
}

/// Stores the byte `value` at `array[index]`; the value is checked first, as CPython checks
/// it, then the position. Returns the exception, or null.
///
/// # Safety
///
/// `array` points to a live bytearray, and `index` and `value` are `int` words.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_bytearray_set(
    array: *mut BytearrayObject,
    index: i64,
    value: i64,
) -> *mut ExceptionObject {
    let value = match byte(value) {
        Ok(value) => value,
        Err(error) => return error,
    };
    // SAFETY: the caller's guarantee; `position` keeps the index inside.
    unsafe {
        match position(array, index, "bytearray index out of range") {
            Ok(at) => {
                room(array)[at] = value;
                core::ptr::null_mut()
            }
            Err(error) => error,
        }
    }
}

/// Returns `repr(array)`: `bytearray(b'...')`, the bytes written as `repr` of a `bytes`
/// writes them.
///
/// # Safety
///
/// `array` points to a live bytearray.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_bytearray_repr(array: *const BytearrayObject) -> *mut StrObject {
    // SAFETY: the caller's guarantee; each part is a live string.
    unsafe {
        let inner = crate::bytes::typy_rt_bytes_repr((*array).buffer);
        let mut text = crate::string::StrBuilder::with_capacity(length(inner) + 11);
        for byte in b"bytearray(" {
            text.push(u32::from(*byte));
        }
        for index in 0..length(inner) {
            text.push(crate::string::code_point(inner, index));
        }
        text.push(u32::from(b')'));
        crate::refcount::typy_rt_decrement(inner.cast());
        text.finish()
    }
}

#[cfg(test)]
mod tests;
