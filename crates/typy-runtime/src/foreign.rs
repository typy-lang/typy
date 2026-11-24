//! Converts values for calls of C functions that `ctypes` declarations describe: an `int`
//! wrapped to 64 bits, an address as an `int`, and a C string as `bytes` (FFI-03, ADR-0047).

use crate::int::value::Integer;
use crate::string::StrObject;
use crate::word::{inline_value, is_inline};

/// Returns the low 64 bits of an `int` word in two's complement, as `ctypes` wraps an `int`
/// to an integer type, or 0 for the word 0, the `None` of an optional `int` that a `c_void_p`
/// passes as null.
///
/// # Safety
///
/// `word` is an `int` word, or 0.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_int_low_bits(word: i64) -> i64 {
    if word == 0 {
        return 0;
    }
    if is_inline(word) {
        return inline_value(word);
    }
    // SAFETY: a word with its lowest bit clear points to a live large integer.
    let value = unsafe { Integer::from_word(word) };
    let low = value.magnitude.as_slice().first().copied().unwrap_or(0);
    let bits = if value.negative {
        low.wrapping_neg()
    } else {
        low
    };
    i64::from_ne_bytes(bits.to_ne_bytes())
}

/// Returns the `int` word of a C `uint64_t`, which the caller owns.
#[unsafe(no_mangle)]
pub extern "C" fn typy_rt_int_from_uint64(value: u64) -> i64 {
    Integer::from_i128(i128::from(value)).into_word()
}

/// Returns an address a C function returned as an `int` word the caller owns, or the word 0,
/// `None`, for null, as `ctypes` gives a `c_void_p` result.
#[unsafe(no_mangle)]
pub extern "C" fn typy_rt_int_from_address(address: *const u8) -> i64 {
    if address.is_null() {
        return 0;
    }
    typy_rt_int_from_uint64(address.addr() as u64)
}

crate::export_float!(unsafe typy_rt_ctypes_float_argument = ctypes_float_argument(word: i64, position: i64));

/// Returns an `int` word as a `double` for a `c_double` or `c_float` argument at `position`,
/// an `int` word counting from 1, or raises `ctypes.ArgumentError` with CPython's text for
/// one too large.
///
/// # Safety
///
/// `word` is an `int` word whose large integer is live.
pub(crate) unsafe fn ctypes_float_argument(word: i64, position: i64) -> crate::Raising<f64> {
    // SAFETY: the caller's guarantee.
    let converted = unsafe { crate::float::convert::int_to_float(word) };
    if converted.exception.is_null() {
        return converted;
    }
    // SAFETY: the `OverflowError` was just raised and nothing else refers to it.
    unsafe { crate::refcount::typy_rt_decrement(converted.exception.cast()) };
    let mut message = crate::message::MessageBuffer::new();
    message.push_str("argument ");
    message.push_number(inline_value(position));
    message.push_str(": OverflowError: int too large to convert to float");
    crate::Raising::error(
        0.0,
        message.raise(crate::exception::ExceptionKind::ArgumentError),
    )
}

/// Returns a new `bytes` of the C string at `text`, up to its zero byte, or null for null,
/// as `ctypes` gives a `c_char_p` result.
///
/// # Safety
///
/// `text` is null, or points to bytes ended by a zero.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_bytes_from_c_string(text: *const u8) -> *mut StrObject {
    if text.is_null() {
        return core::ptr::null_mut();
    }
    // SAFETY: the caller's guarantee: the bytes run to a zero.
    let data = unsafe { core::ffi::CStr::from_ptr(text.cast()) }.to_bytes();
    crate::bytes::from_slice(data)
}

#[cfg(test)]
mod tests;
