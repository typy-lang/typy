//! Tests of `bytearray`: construction, adoption of a result, item and slice access, the list
//! methods, the operators and the text, each against CPython 3.15's results.

use super::{
    BytearrayObject, bytes_of, from_data, typy_rt_bytearray_adopt, typy_rt_bytearray_append,
    typy_rt_bytearray_clear, typy_rt_bytearray_concat, typy_rt_bytearray_contains_byte,
    typy_rt_bytearray_copy, typy_rt_bytearray_delete, typy_rt_bytearray_delete_slice,
    typy_rt_bytearray_extend, typy_rt_bytearray_extend_list, typy_rt_bytearray_from_bytes,
    typy_rt_bytearray_from_list, typy_rt_bytearray_get, typy_rt_bytearray_insert,
    typy_rt_bytearray_length, typy_rt_bytearray_new, typy_rt_bytearray_pop,
    typy_rt_bytearray_remove, typy_rt_bytearray_repeat, typy_rt_bytearray_repeat_in_place,
    typy_rt_bytearray_repr, typy_rt_bytearray_reverse, typy_rt_bytearray_set,
    typy_rt_bytearray_set_slice, typy_rt_bytearray_slice, typy_rt_bytearray_to_bytes,
};
use crate::exception::ExceptionObject;
use crate::list::{ItemKind, new_list, typy_rt_list_append};
use crate::slice::{START, STEP, STOP};
use crate::word::{inline, inline_value};

fn word(value: i64) -> i64 {
    inline(value).unwrap()
}

/// Returns the bytes the bytearray holds now.
fn held(array: *const BytearrayObject) -> Vec<u8> {
    // SAFETY: every bytearray in these tests is live.
    unsafe { bytes_of(array) }.to_vec()
}

/// Returns the text of a string.
fn text(string: *const crate::StrObject) -> String {
    // SAFETY: the string is live and every index is in bounds.
    (0..unsafe { crate::string::length(string) })
        .map(|index| char::from_u32(unsafe { crate::string::code_point(string, index) }).unwrap())
        .collect()
}

/// Returns the message of an exception the runtime raised.
fn raised(error: *mut ExceptionObject) -> String {
    assert!(!error.is_null(), "no exception");
    // SAFETY: the exception is live, and the runtime gives each it raises a message.
    text(unsafe { (*error).message })
}

#[test]
fn the_layout_is_header_buffer_and_capacity() {
    use std::mem::offset_of;
    assert_eq!(
        (
            offset_of!(BytearrayObject, buffer),
            offset_of!(BytearrayObject, capacity)
        ),
        (8, 16)
    );
}

#[test]
fn a_bytearray_is_made_of_zeros_bytes_or_a_list_of_ints() {
    // SAFETY: the arguments are live and the words are `int` words.
    unsafe {
        let zeros = typy_rt_bytearray_new(word(3));
        assert_eq!(held(zeros.value), [0, 0, 0]);
        assert_eq!(
            raised(typy_rt_bytearray_new(word(-1)).exception),
            "negative count"
        );
        let source = crate::bytes::from_slice(b"abc");
        let copied = typy_rt_bytearray_from_bytes(source);
        assert_eq!(held(copied), b"abc");
        let values = new_list(ItemKind::Int, 0);
        for value in [1, 2, 255] {
            typy_rt_list_append(values, word(value).cast_unsigned());
        }
        assert_eq!(held(typy_rt_bytearray_from_list(values).value), [1, 2, 255]);
        typy_rt_list_append(values, word(300).cast_unsigned());
        assert_eq!(
            raised(typy_rt_bytearray_from_list(values).exception),
            "byte must be in range(0, 256)"
        );
    }
}

#[test]
fn a_result_held_once_becomes_the_buffer_and_one_held_twice_is_copied() {
    // SAFETY: the `bytes` are live and the caller holds the reference each call gives up.
    unsafe {
        let only = crate::bytes::from_slice(b"xy");
        let adopted = typy_rt_bytearray_adopt(only);
        assert!(core::ptr::eq((*adopted).buffer, only));
        let shared = crate::bytes::from_slice(b"zz");
        crate::refcount::typy_rt_increment(shared.cast());
        let copied = typy_rt_bytearray_adopt(shared);
        assert!(!core::ptr::eq((*copied).buffer, shared));
        assert_eq!(held(copied), b"zz");
        assert_eq!((*shared).header.reference_count, 1);
    }
}

#[test]
fn items_are_read_and_written_with_cpythons_errors() {
    let array = from_data(b"abc");
    // SAFETY: the bytearray is live and the words are `int` words.
    unsafe {
        assert_eq!(
            inline_value(typy_rt_bytearray_get(array, word(-1)).value),
            99
        );
        assert_eq!(
            raised(typy_rt_bytearray_get(array, word(3)).exception),
            "bytearray index out of range"
        );
        assert!(typy_rt_bytearray_set(array, word(0), word(65)).is_null());
        // The value is checked before the position, as CPython checks it.
        assert_eq!(
            raised(typy_rt_bytearray_set(array, word(10), word(300))),
            "byte must be in range(0, 256)"
        );
        assert_eq!(
            raised(typy_rt_bytearray_set(array, word(10), word(1))),
            "bytearray index out of range"
        );
        assert!(typy_rt_bytearray_delete(array, word(1)).is_null());
        assert_eq!(held(array), b"Ac");
        assert_eq!(inline_value(typy_rt_bytearray_length(array)), 2);
    }
}

#[test]
fn the_list_methods_change_the_bytearray_in_place() {
    let array = from_data(b"abc");
    // SAFETY: the bytearray is live and the words are `int` words.
    unsafe {
        assert!(typy_rt_bytearray_append(array, word(100)).is_null());
        assert!(typy_rt_bytearray_insert(array, word(-100), word(95)).is_null());
        assert!(typy_rt_bytearray_insert(array, word(99), word(33)).is_null());
        assert_eq!(held(array), b"_abcd!");
        assert_eq!(
            inline_value(typy_rt_bytearray_pop(array, word(-1)).value),
            33
        );
        assert_eq!(
            inline_value(typy_rt_bytearray_pop(array, word(0)).value),
            95
        );
        assert_eq!(
            raised(typy_rt_bytearray_pop(array, word(9)).exception),
            "pop index out of range"
        );
        assert!(typy_rt_bytearray_remove(array, word(98)).is_null());
        assert_eq!(
            raised(typy_rt_bytearray_remove(array, word(98))),
            "value not found in bytearray"
        );
        typy_rt_bytearray_reverse(array);
        assert_eq!(held(array), b"dca");
        let copy = typy_rt_bytearray_copy(array);
        typy_rt_bytearray_clear(array);
        assert_eq!((held(array), held(copy)), (Vec::new(), b"dca".to_vec()));
        assert_eq!(
            raised(typy_rt_bytearray_pop(array, word(-1)).exception),
            "pop from empty bytearray"
        );
        assert_eq!(
            raised(typy_rt_bytearray_append(array, word(-1))),
            "byte must be in range(0, 256)"
        );
    }
}

#[test]
fn extending_reads_bytes_lists_and_the_bytearray_itself() {
    let array = from_data(b"ab");
    // SAFETY: the arguments are live and the words are `int` words.
    unsafe {
        typy_rt_bytearray_extend(array, crate::bytes::from_slice(b"cd"));
        typy_rt_bytearray_extend(array, (*array).buffer);
        assert_eq!(held(array), b"abcdabcd");
        let values = new_list(ItemKind::Int, 0);
        for value in [120, 256, 121] {
            typy_rt_list_append(values, word(value).cast_unsigned());
        }
        // The items before the one out of range stay, as in CPython.
        assert_eq!(
            raised(typy_rt_bytearray_extend_list(array, values)),
            "byte must be in range(0, 256)"
        );
        assert_eq!(held(array), b"abcdabcdx");
        for _ in 0..1000 {
            typy_rt_bytearray_append(array, word(46));
        }
        assert_eq!(held(array).len(), 1009);
        assert!((*array).capacity >= 1009);
    }
}

#[test]
fn slices_are_read_written_and_deleted_as_cpython_does() {
    let all = word(START | STOP | STEP);
    // SAFETY: the arguments are live and the words are `int` words.
    unsafe {
        let array = from_data(b"abcdef");
        let middle = typy_rt_bytearray_slice(array, word(1), word(4), 0, word(START | STOP));
        assert_eq!(held(middle.value), b"bcd");
        let backwards = typy_rt_bytearray_slice(array, 0, 0, word(-2), word(STEP));
        assert_eq!(held(backwards.value), b"fdb");
        let replaced = crate::bytes::from_slice(b"XYZW");
        assert!(
            typy_rt_bytearray_set_slice(array, word(1), word(3), 0, word(START | STOP), replaced)
                .is_null()
        );
        assert_eq!(held(array), b"aXYZWdef");
        assert!(
            typy_rt_bytearray_set_slice(
                array,
                word(0),
                word(2),
                0,
                word(START | STOP),
                (*array).buffer
            )
            .is_null()
        );
        assert_eq!(held(array), b"aXYZWdefYZWdef");
        let error = typy_rt_bytearray_set_slice(
            array,
            word(0),
            word(6),
            word(2),
            all,
            crate::bytes::from_slice(b"q"),
        );
        assert_eq!(
            raised(error),
            "attempt to assign bytes of size 1 to extended slice of size 3"
        );
        assert!(typy_rt_bytearray_delete_slice(array, word(0), word(14), word(2), all).is_null());
        assert_eq!(held(array), b"XZdfZdf");
        let backwards = from_data(b"0123456789");
        assert!(
            typy_rt_bytearray_delete_slice(backwards, word(8), 0, word(-3), word(START | STEP))
                .is_null()
        );
        assert_eq!(held(backwards), b"0134679");
    }
}

#[test]
fn operators_make_new_bytearrays_or_change_one_in_place() {
    let array = from_data(b"ab");
    // SAFETY: the arguments are live and the words are `int` words.
    unsafe {
        let joined = typy_rt_bytearray_concat(array, crate::bytes::from_slice(b"x"));
        assert_eq!(held(joined), b"abx");
        assert_eq!(
            held(typy_rt_bytearray_repeat(array, word(2)).value),
            b"abab"
        );
        assert_eq!(held(typy_rt_bytearray_repeat(array, word(-3)).value), b"");
        assert!(typy_rt_bytearray_repeat_in_place(array, word(3)).is_null());
        assert_eq!(held(array), b"ababab");
        assert!(typy_rt_bytearray_contains_byte(array, word(98)).value);
        assert!(!typy_rt_bytearray_contains_byte(array, word(0)).value);
        let frozen = typy_rt_bytearray_to_bytes(array);
        typy_rt_bytearray_clear(array);
        assert_eq!(super::data_of(frozen), b"ababab");
    }
}

#[test]
fn the_text_is_bytearray_around_the_bytes_text() {
    // SAFETY: each bytearray is new.
    let written = |data: &[u8]| text(unsafe { typy_rt_bytearray_repr(from_data(data)) });
    assert_eq!(written(b"ab"), "bytearray(b'ab')");
    assert_eq!(written(b"it's"), "bytearray(b\"it's\")");
    assert_eq!(written(b""), "bytearray(b'')");
}

#[test]
fn a_large_int_as_a_value_or_a_position_is_refused() {
    let large = crate::int::value::Integer::from_i128(1 << 70).into_word();
    let array = from_data(b"ab");
    // SAFETY: the bytearray is live and the words are `int` words.
    unsafe {
        assert_eq!(
            raised(typy_rt_bytearray_append(array, large)),
            "byte must be in range(0, 256)"
        );
        assert_eq!(
            raised(typy_rt_bytearray_get(array, large).exception),
            "cannot fit 'int' into an index-sized integer"
        );
    }
}

#[test]
fn an_extended_slice_takes_as_many_bytes_as_it_selects() {
    let array = from_data(b"abcdef");
    let all = word(START | STOP | STEP);
    // SAFETY: the arguments are live and the words are `int` words.
    unsafe {
        let three = crate::bytes::from_slice(b"XYZ");
        assert!(
            typy_rt_bytearray_set_slice(array, word(0), word(6), word(2), all, three).is_null()
        );
        assert_eq!(held(array), b"XbYdZf");
    }
}

#[test]
fn a_repeat_too_long_raises_and_leaves_the_bytearray_as_it_was() {
    // Eight bytes repeated 2**62 - 1 times is more than a size holds.
    let array = from_data(b"abcdefgh");
    let huge = word((1 << 62) - 1);
    // SAFETY: the bytearray is live and the words are `int` words.
    unsafe {
        assert_eq!(
            raised(typy_rt_bytearray_repeat(array, huge).exception),
            "repeated string is too long"
        );
        assert_eq!(
            raised(typy_rt_bytearray_repeat_in_place(array, huge)),
            "repeated string is too long"
        );
        assert_eq!(held(array), b"abcdefgh");
    }
}
