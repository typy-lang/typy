//! Tests of the conversions of values for calls of C functions.

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
