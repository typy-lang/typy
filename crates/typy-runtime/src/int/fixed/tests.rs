//! Tests of converting `int` words to and from C's `int64_t`.

use super::{typy_rt_int_from_int64, typy_rt_int_into_int64};

#[test]
fn every_int64_comes_back_as_it_went() {
    for value in [
        0,
        1,
        -1,
        1 << 61,
        -(1 << 61),
        (1 << 62) - 1,
        1 << 62,
        i64::MAX,
        i64::MIN,
    ] {
        let word = typy_rt_int_from_int64(value);
        // SAFETY: the word is the caller's own, handed back.
        let back = unsafe { typy_rt_int_into_int64(word) };
        assert!(back.exception.is_null(), "{value}");
        assert_eq!(back.value, value);
    }
}

#[test]
fn an_int_outside_int64_raises_overflow_error() {
    let word = crate::int::value::Integer::from_i128(i128::from(i64::MAX) + 1).into_word();
    // SAFETY: the word is the caller's own, handed over.
    let result = unsafe { typy_rt_int_into_int64(word) };
    assert!(!result.exception.is_null());
    // SAFETY: the exception was just raised and nothing else refers to it.
    unsafe {
        assert_eq!(
            (*result.exception).kind(),
            crate::ExceptionKind::OverflowError
        );
        crate::refcount::typy_rt_decrement(result.exception.cast());
    }
}
