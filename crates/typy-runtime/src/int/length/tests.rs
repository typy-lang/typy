//! Tests of the check of the `int` a class's `__len__` returns.

use super::typy_rt_int_length;
use crate::word::inline;

#[test]
fn a_length_is_kept_and_a_negative_one_raises() {
    let word = |value| inline(value).unwrap();
    // SAFETY: inline words are live `int` words.
    unsafe {
        let kept = typy_rt_int_length(word(3));
        assert!(kept.exception.is_null());
        assert_eq!(kept.value, word(3));
        let raised = typy_rt_int_length(word(-1));
        assert!(!raised.exception.is_null());
        crate::refcount::typy_rt_decrement(raised.exception.cast());
        // A length past `sys.maxsize` raises `OverflowError`.
        let large = super::Integer::from_i128(1 << 100).into_word();
        let raised = typy_rt_int_length(large);
        assert!(!raised.exception.is_null());
        crate::refcount::typy_rt_decrement(raised.exception.cast());
    }
}
