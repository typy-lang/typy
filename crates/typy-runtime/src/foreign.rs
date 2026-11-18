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
mod tests {
    use super::{
        typy_rt_bytes_from_c_string, typy_rt_int_from_address, typy_rt_int_from_uint64,
        typy_rt_int_low_bits,
    };
    use crate::int::value::Integer;

    #[test]
    fn the_low_bits_of_an_int_wrap_as_ctypes_wraps_it() {
        for (value, bits) in [
            (0_i128, 0_i64),
            (300, 300),
            (-1, -1),
            (1 << 62, 1 << 62),
            (-(1 << 62) - 1, -(1 << 62) - 1),
            (1 << 64, 0),
            ((1 << 64) + 3, 3),
            (-(1 << 64) - 3, -3),
            (i128::from(u64::MAX), -1),
        ] {
            let word = Integer::from_i128(value).into_word();
            // SAFETY: the word was just made.
            let low = unsafe { typy_rt_int_low_bits(word) };
            assert_eq!(low, bits, "{value}");
            // SAFETY: the word is the test's own, dropped once.
            unsafe { crate::refcount::typy_rt_int_decrement(word) };
        }
        // SAFETY: 0 is the word `None` of an optional `int`.
        assert_eq!(unsafe { typy_rt_int_low_bits(0) }, 0);
    }

    #[test]
    fn an_unsigned_result_and_an_address_become_ints() {
        let word = typy_rt_int_from_uint64(u64::MAX);
        // SAFETY: the word was just made and is dropped once.
        unsafe {
            assert_eq!(
                Integer::from_word(word).to_i128(),
                Some(i128::from(u64::MAX))
            );
            crate::refcount::typy_rt_int_decrement(word);
        }
        assert_eq!(typy_rt_int_from_address(core::ptr::null()), 0);
        assert_eq!(
            typy_rt_int_from_address(core::ptr::without_provenance(48)),
            crate::word::inline(48).unwrap_or(0)
        );
    }

    #[test]
    fn an_int_too_large_for_a_double_is_a_ctypes_argument_error() {
        let fits = crate::int::value::Integer::from_i128(1 << 70).into_word();
        // SAFETY: the words are the test's own, dropped once.
        unsafe {
            let converted = super::ctypes_float_argument(fits, crate::word::inline(1).unwrap_or(0));
            assert!(converted.exception.is_null());
            // 2 to the 70th is exact in a `double`; `powi` is not, under Miri.
            assert_eq!(
                converted.value.to_bits(),
                1_180_591_620_717_411_303_424.0_f64.to_bits()
            );
            crate::refcount::typy_rt_int_decrement(fits);
        }
        let mut huge = crate::int::value::Integer::from_i128(1);
        for _ in 0..1100 {
            huge = crate::int::value::Integer::from_i128(2).multiply(&huge);
        }
        let word = huge.into_word();
        // SAFETY: as above.
        unsafe {
            let converted = super::ctypes_float_argument(word, crate::word::inline(2).unwrap_or(0));
            assert!(!converted.exception.is_null());
            let shown = crate::exception::typy_rt_exception_str(converted.exception.cast()).value;
            let text: String = (0..crate::string::length(shown))
                .filter_map(|index| char::from_u32(crate::string::code_point(shown, index)))
                .collect();
            assert_eq!(
                text,
                "argument 2: OverflowError: int too large to convert to float"
            );
            crate::refcount::typy_rt_decrement(shown.cast());
            assert_eq!(
                (*converted.exception).kind(),
                crate::ExceptionKind::ArgumentError
            );
            crate::refcount::typy_rt_decrement(converted.exception.cast());
            crate::refcount::typy_rt_int_decrement(word);
        }
    }

    #[test]
    fn a_c_string_becomes_bytes_and_null_none() {
        // SAFETY: a null pointer, then a string ended by a zero.
        unsafe {
            assert!(typy_rt_bytes_from_c_string(core::ptr::null()).is_null());
            let bytes = typy_rt_bytes_from_c_string(c"hi".as_ptr().cast());
            assert_eq!(crate::string::length(bytes), 2);
            crate::refcount::typy_rt_decrement(bytes.cast());
        }
    }
}
