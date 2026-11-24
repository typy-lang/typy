//! Tests of reference counting.

use super::{typy_rt_decrement, typy_rt_increment, typy_rt_int_decrement, typy_rt_int_increment};
use crate::header::{IMMORTAL, ObjectHeader, STR_TYPE_ID};

#[test]
fn immortal_objects_are_never_counted() {
    let mut header = ObjectHeader {
        reference_count: IMMORTAL,
        type_id: STR_TYPE_ID,
    };
    // SAFETY: the header is live for the whole test.
    unsafe {
        typy_rt_increment(&raw mut header);
        typy_rt_decrement(&raw mut header);
    }
    assert_eq!(header.reference_count, IMMORTAL);
}

#[test]
fn counts_go_up_and_down_and_the_last_reference_frees() {
    let object = crate::string::from_str("abc");
    // SAFETY: the string is live until its last reference is dropped at the end.
    unsafe {
        typy_rt_increment(object.cast());
        assert_eq!((*object).header.reference_count, 2);
        typy_rt_decrement(object.cast());
        assert_eq!((*object).header.reference_count, 1);
        typy_rt_decrement(object.cast());
    }
}

#[test]
fn none_of_an_optional_value_is_not_counted() {
    // SAFETY: the null address and the word 0 stand for `None` and are never dereferenced.
    unsafe {
        typy_rt_increment(core::ptr::null_mut());
        typy_rt_decrement(core::ptr::null_mut());
        typy_rt_int_increment(0);
        typy_rt_int_decrement(0);
    }
}

#[test]
fn inline_int_words_are_not_counted() {
    // SAFETY: odd words are inline integers and are never dereferenced.
    unsafe {
        typy_rt_int_increment(7);
        typy_rt_int_decrement(7);
    }
}

#[test]
fn large_int_words_are_counted_like_objects() {
    let word = crate::int::value::Integer::from_i128(1 << 70).into_word();
    let object = crate::int::object_of(word);
    // SAFETY: the integer is live until its last reference is dropped at the end.
    unsafe {
        typy_rt_int_increment(word);
        assert_eq!((*object).header.reference_count, 2);
        typy_rt_int_decrement(word);
        assert_eq!((*object).header.reference_count, 1);
        typy_rt_int_decrement(word);
    }
}
