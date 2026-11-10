//! `bytes`: an immutable sequence of integers from 0 to 255 (DAT-06). A `bytes` is a `str`
//! object of one byte per code point under its own type ID, so it shares the string
//! algorithms, and these functions give it its own results: items that are `int`, text in
//! the form `b'...'`, and new objects of type `bytes`.

mod methods;

use crate::Raising;
use crate::exception::{ExceptionKind, raise};
use crate::header::BYTES_TYPE_ID;
use crate::slice::clamped;
use crate::string::{StrKind, StrObject, code_point, length, new_string, set_code_point};
use crate::word::{inline, inline_value, is_inline};

/// Returns a new `bytes` of `count` bytes, not yet written.
pub(crate) fn new_bytes(count: usize) -> *mut StrObject {
    let object = new_string(count, StrKind::Latin1);
    // SAFETY: the object was just made and nothing else refers to it.
    unsafe { (*object).header.type_id = BYTES_TYPE_ID };
    object
}

/// Gives a new string the `bytes` type, for the results of the string functions on `bytes`.
///
/// # Safety
///
/// `object` points to a new string of one byte per code point that nothing else refers to.
unsafe fn as_bytes(object: *mut StrObject) -> *mut StrObject {
    // SAFETY: the caller's guarantee.
    unsafe { (*object).header.type_id = BYTES_TYPE_ID };
    object
}

/// Returns `left + right`.
///
/// # Safety
///
/// Both point to live `bytes`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_bytes_concat(
    left: *const StrObject,
    right: *const StrObject,
) -> *mut StrObject {
    // SAFETY: the caller's guarantee; the result is new.
    unsafe { as_bytes(crate::string::typy_rt_str_concat(left, right)) }
}

/// Returns `data * count`.
///
/// # Safety
///
/// `data` points to a live `bytes`, and `count` is an `int` word.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_bytes_repeat(
    data: *const StrObject,
    count: i64,
) -> Raising<*mut StrObject> {
    // SAFETY: the caller's guarantee; the result is new.
    unsafe {
        let repeated = crate::string::typy_rt_str_repeat(data, count);
        if repeated.exception.is_null() {
            Raising::ok(as_bytes(repeated.value))
        } else {
            repeated
        }
    }
}

/// Returns `data[start:stop:step]`, a new `bytes`; `present` says which bounds are given.
///
/// # Safety
///
/// `data` points to a live `bytes`, and the bounds are `int` words.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_bytes_slice(
    data: *const StrObject,
    start: i64,
    stop: i64,
    stride: i64,
    present: i64,
) -> Raising<*mut StrObject> {
    // SAFETY: the caller's guarantee; the result is new.
    unsafe {
        let sliced = crate::string::typy_rt_str_slice(data, start, stop, stride, present);
        if sliced.exception.is_null() {
            Raising::ok(as_bytes(sliced.value))
        } else {
            sliced
        }
    }
}

/// Returns `data[index]`, the byte at a position counted from the end when negative, as an
/// `int` word; one outside raises `IndexError: index out of range`.
///
/// # Safety
///
/// `data` points to a live `bytes`, and `index` is an `int` word.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_bytes_get(data: *const StrObject, index: i64) -> Raising<i64> {
    if !is_inline(index) {
        let error = raise(
            ExceptionKind::IndexError,
            "cannot fit 'int' into an index-sized integer",
        );
        return Raising::error(1, error);
    }
    // SAFETY: the caller passes a live `bytes`; the position is checked against its length.
    unsafe {
        let size = i64::try_from(length(data)).unwrap_or(i64::MAX);
        let position = inline_value(index);
        let position = if position < 0 {
            position + size
        } else {
            position
        };
        if !(0..size).contains(&position) {
            return Raising::error(1, raise(ExceptionKind::IndexError, "index out of range"));
        }
        let byte = code_point(data, usize::try_from(position).unwrap_or(0));
        Raising::ok(inline(i64::from(byte)).unwrap_or(1))
    }
}

/// Returns `len(data)`, as an `int` word.
///
/// # Safety
///
/// `data` points to a live `bytes`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_bytes_length(data: *const StrObject) -> i64 {
    // SAFETY: the caller's guarantee.
    unsafe { crate::string::typy_rt_str_length(data) }
}

/// Returns whether two `bytes` stand in the comparison `comparison`, an `int` word: byte by
/// byte, then by length, as two strings of one byte per code point compare.
///
/// # Safety
///
/// Both point to live `bytes`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_bytes_compare(
    left: *const StrObject,
    right: *const StrObject,
    comparison: i64,
) -> bool {
    // SAFETY: the caller's guarantee.
    unsafe { crate::string::typy_rt_str_compare(left, right, comparison) }
}

/// Returns `needle in data` for `bytes`: whether the bytes of `needle` appear together.
///
/// # Safety
///
/// Both point to live `bytes`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_bytes_contains(
    data: *const StrObject,
    needle: *const StrObject,
) -> bool {
    // SAFETY: the caller's guarantee.
    unsafe { crate::string::typy_rt_str_contains(data, needle) }
}

/// Returns `byte in data` for an `int`; one outside 0 to 255 raises `ValueError`.
///
/// # Safety
///
/// `data` points to a live `bytes`, and `byte` is an `int` word.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_bytes_contains_byte(
    data: *const StrObject,
    byte: i64,
) -> Raising<bool> {
    let value = clamped(byte);
    if !(0..=255).contains(&value) {
        let error = raise(ExceptionKind::ValueError, "byte must be in range(0, 256)");
        return Raising::error(false, error);
    }
    let value = u32::try_from(value).unwrap_or(0);
    // SAFETY: the caller passes a live `bytes`; every index stays inside it.
    unsafe { Raising::ok((0..length(data)).any(|index| code_point(data, index) == value)) }
}

/// Returns `bytes(count)`: `count` zero bytes; a negative count raises `ValueError`.
///
/// # Safety
///
/// `count` is an `int` word.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_bytes_zeros(count: i64) -> Raising<*mut StrObject> {
    let count = clamped(count);
    if count < 0 {
        return Raising::error(
            core::ptr::null_mut(),
            raise(ExceptionKind::ValueError, "negative count"),
        );
    }
    let count = usize::try_from(count).unwrap_or(usize::MAX);
    let data = new_bytes(count);
    for index in 0..count {
        // SAFETY: the new object has room for `count` bytes.
        unsafe { set_code_point(data, index, 0) };
    }
    Raising::ok(data)
}

/// Returns `bytes(values)` for a list of `int` items; an item outside 0 to 255 raises
/// `ValueError: bytes must be in range(0, 256)`.
///
/// # Safety
///
/// `values` points to a live list of `int` items.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_bytes_from_list(
    values: *const crate::ListObject,
) -> Raising<*mut StrObject> {
    // SAFETY: the caller passes a live list of `int` words.
    unsafe {
        let count = (*values).length;
        let in_range = (0..count)
            .all(|index| (0..=255).contains(&clamped((*values).item(index).cast_signed())));
        if !in_range {
            let error = raise(ExceptionKind::ValueError, "bytes must be in range(0, 256)");
            return Raising::error(core::ptr::null_mut(), error);
        }
        let data = new_bytes(count);
        for index in 0..count {
            let byte =
                u32::try_from(inline_value((*values).item(index).cast_signed())).unwrap_or(0);
            set_code_point(data, index, byte);
        }
        Raising::ok(data)
    }
}

/// Returns `repr(data)`: `b'...'`, in double quotes when the bytes hold a single quote and no
/// double quote, with `\t`, `\n`, `\r`, `\\`, the quote and every byte outside 0x20 to 0x7e
/// escaped, as CPython writes it.
///
/// # Safety
///
/// `data` points to a live `bytes`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_bytes_repr(data: *const StrObject) -> *mut StrObject {
    // SAFETY: the caller passes a live `bytes`; every index stays inside it.
    unsafe {
        let size = length(data);
        let has = |byte: u32| (0..size).any(|index| code_point(data, index) == byte);
        let quote = if has(0x27) && !has(0x22) { 0x22 } else { 0x27 };
        let mut text = crate::string::StrBuilder::with_capacity(size + 3);
        text.push(u32::from(b'b'));
        text.push(quote);
        for index in 0..size {
            let byte = code_point(data, index);
            let mut put =
                |escaped: &[u8]| escaped.iter().for_each(|each| text.push(u32::from(*each)));
            match byte {
                0x09 => put(b"\\t"),
                0x0a => put(b"\\n"),
                0x0d => put(b"\\r"),
                0x5c => put(b"\\\\"),
                _ if byte == quote => {
                    put(b"\\");
                    text.push(byte);
                }
                0x20..=0x7e => text.push(byte),
                _ => {
                    let digits = b"0123456789abcdef";
                    let high = usize::try_from(byte >> 4).unwrap_or(0);
                    let low = usize::try_from(byte & 0xf).unwrap_or(0);
                    put(&[b'\\', b'x', digits[high], digits[low]]);
                }
            }
        }
        text.push(quote);
        text.finish()
    }
}

#[cfg(test)]
mod tests;
