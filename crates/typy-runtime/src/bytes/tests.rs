//! Tests of `bytes`: text, items, containment, construction and the type of results.

use super::{
    new_bytes, typy_rt_bytes_compare, typy_rt_bytes_concat, typy_rt_bytes_contains,
    typy_rt_bytes_contains_byte, typy_rt_bytes_from_list, typy_rt_bytes_get, typy_rt_bytes_length,
    typy_rt_bytes_repeat, typy_rt_bytes_repr, typy_rt_bytes_slice, typy_rt_bytes_zeros,
};
use crate::header::BYTES_TYPE_ID;
use crate::int::value::Integer;
use crate::list::{ItemKind, new_list};
use crate::string::{StrObject, code_point, length, set_code_point};
use crate::word::{inline, inline_value};

fn word(value: i64) -> i64 {
    inline(value).unwrap()
}

fn bytes(values: &[u8]) -> *mut StrObject {
    let data = new_bytes(values.len());
    for (index, value) in values.iter().enumerate() {
        // SAFETY: the new object has room for every byte.
        unsafe { set_code_point(data, index, u32::from(*value)) };
    }
    data
}

fn text(string: *const StrObject) -> String {
    // SAFETY: the string is live and every index is in bounds.
    (0..unsafe { length(string) })
        .map(|index| char::from_u32(unsafe { code_point(string, index) }).unwrap())
        .collect()
}

fn values(data: *const StrObject) -> Vec<u32> {
    // SAFETY: the data is live and every index is in bounds.
    unsafe {
        assert_eq!((*data).header.type_id, BYTES_TYPE_ID);
        (0..length(data))
            .map(|index| code_point(data, index))
            .collect()
    }
}

#[test]
fn bytes_are_written_as_cpython_writes_them() {
    let cases: [(&[u8], &str); 7] = [
        (&[], "b''"),
        (&[97, 98, 99], "b'abc'"),
        (&[105, 116, 39, 115], "b\"it's\""),
        (&[97, 34, 98], "b'a\"b'"),
        (&[39, 34], "b'\\'\"'"),
        (
            &[0, 127, 128, 255, 9, 10, 13, 92],
            "b'\\x00\\x7f\\x80\\xff\\t\\n\\r\\\\'",
        ),
        (&[32, 126], "b' ~'"),
    ];
    for (data, expected) in cases {
        // SAFETY: the bytes are live.
        assert_eq!(text(unsafe { typy_rt_bytes_repr(bytes(data)) }), expected);
    }
}

#[test]
fn items_and_containment_are_ints() {
    let data = bytes(&[0, 200, 7]);
    let huge = Integer::from_i128(1 << 70).into_word();
    // SAFETY: the bytes and words are live.
    unsafe {
        assert_eq!(inline_value(typy_rt_bytes_get(data, word(1)).value), 200);
        assert_eq!(inline_value(typy_rt_bytes_get(data, word(-1)).value), 7);
        assert!(!typy_rt_bytes_get(data, word(3)).exception.is_null());
        assert!(!typy_rt_bytes_get(data, huge).exception.is_null());
        assert!(typy_rt_bytes_contains_byte(data, word(200)).value);
        assert!(!typy_rt_bytes_contains_byte(data, word(9)).value);
        assert!(
            !typy_rt_bytes_contains_byte(data, word(256))
                .exception
                .is_null()
        );
        assert!(!typy_rt_bytes_contains_byte(data, huge).exception.is_null());
    }
}

#[test]
fn construction_checks_each_byte() {
    let list = new_list(ItemKind::Int, 0);
    for value in [1, 255] {
        // SAFETY: the list is live and an inline word needs no reference.
        unsafe { crate::list::typy_rt_list_append(list, word(value).cast_unsigned()) };
    }
    // SAFETY: the list and words are live.
    unsafe {
        assert_eq!(values(typy_rt_bytes_from_list(list).value), [1, 255]);
        crate::list::typy_rt_list_append(list, word(256).cast_unsigned());
        assert!(!typy_rt_bytes_from_list(list).exception.is_null());
        assert_eq!(values(typy_rt_bytes_zeros(word(3)).value), [0, 0, 0]);
        assert!(!typy_rt_bytes_zeros(word(-1)).exception.is_null());
    }
}

#[test]
fn joined_repeated_and_sliced_bytes_stay_bytes() {
    let (left, right) = (bytes(&[1, 2]), bytes(&[3]));
    // SAFETY: the bytes are live.
    unsafe {
        assert_eq!(values(typy_rt_bytes_concat(left, right)), [1, 2, 3]);
        assert_eq!(values(typy_rt_bytes_repeat(right, word(2)).value), [3, 3]);
        assert!(
            !typy_rt_bytes_repeat(right, Integer::from_i128(1 << 70).into_word())
                .exception
                .is_null()
        );
        let sliced = typy_rt_bytes_slice(left, word(0), word(0), word(-1), word(4));
        assert_eq!(values(sliced.value), [2, 1]);
        assert!(
            !typy_rt_bytes_slice(left, word(0), word(0), word(0), word(4))
                .exception
                .is_null()
        );
    }
}

#[test]
fn bytes_measure_compare_and_contain_as_strings_of_bytes_do() {
    let (left, right) = (bytes(&[1, 200]), bytes(&[1, 201]));
    // SAFETY: the bytes are live.
    unsafe {
        assert_eq!(inline_value(typy_rt_bytes_length(left)), 2);
        assert!(typy_rt_bytes_compare(left, right, word(2)));
        assert!(typy_rt_bytes_contains(left, bytes(&[200])));
        assert!(!typy_rt_bytes_contains(left, bytes(&[201])));
    }
}
