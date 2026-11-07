//! Tests of the string operations.

use super::{
    typy_rt_str_compare, typy_rt_str_concat, typy_rt_str_index, typy_rt_str_length,
    typy_rt_str_repeat,
};
use crate::string::{StrKind, StrObject, code_point, from_str, length};
use crate::word::{inline, inline_value};

fn text(string: *const StrObject) -> String {
    // SAFETY: the string is live and every index is in bounds.
    (0..unsafe { length(string) })
        .map(|index| char::from_u32(unsafe { code_point(string, index) }).unwrap())
        .collect()
}

fn word(value: i64) -> i64 {
    inline(value).unwrap()
}

#[test]
fn concatenation_widens_to_the_wider_kind() {
    // SAFETY: both strings are live.
    let joined = unsafe { typy_rt_str_concat(from_str("ab"), from_str("\u{3b1}")) };
    assert_eq!(text(joined), "ab\u{3b1}");
    // SAFETY: the result was just created.
    assert_eq!(unsafe { (*joined).kind }, StrKind::Ucs2);
}

#[test]
fn repetition_follows_python_for_every_count() {
    let source = from_str("ab");
    for (count, expected) in [(3, "ababab"), (1, "ab"), (0, ""), (-2, "")] {
        // SAFETY: the source is live.
        let result = unsafe { typy_rt_str_repeat(source, word(count)) };
        assert!(result.exception.is_null());
        assert_eq!(text(result.value), expected);
    }
}

#[test]
fn indexing_counts_negative_indexes_from_the_end_and_raises_outside() {
    let source = from_str("a\u{e9}c");
    for (index, expected) in [(0, "a"), (1, "\u{e9}"), (-1, "c"), (-3, "a")] {
        // SAFETY: the source is live.
        let result = unsafe { typy_rt_str_index(source, word(index)) };
        assert_eq!(text(result.value), expected);
    }
    for index in [3, -4, 100] {
        // SAFETY: the source is live.
        let result = unsafe { typy_rt_str_index(source, word(index)) };
        assert!(!result.exception.is_null(), "{index}");
        // SAFETY: the exception is live; its message is a live string.
        unsafe {
            assert_eq!((*result.exception).kind, crate::ExceptionKind::IndexError);
            assert_eq!(
                text((*result.exception).message),
                "string index out of range"
            );
        }
    }
}

#[test]
fn length_counts_code_points() {
    // SAFETY: the string is live.
    let length = unsafe { typy_rt_str_length(from_str("a\u{1f600}")) };
    assert_eq!(inline_value(length), 2);
}

#[test]
fn comparison_orders_by_code_point_then_length() {
    let cases = [
        ("a", "b", 2, true),
        ("b", "a", 2, false),
        ("ab", "a", 4, true),
        ("a", "a", 0, true),
        ("a", "a", 1, false),
        ("\u{e9}", "z", 4, true),
    ];
    for (left, right, code, expected) in cases {
        // SAFETY: both strings are live.
        let result = unsafe { typy_rt_str_compare(from_str(left), from_str(right), word(code)) };
        assert_eq!(result, expected, "{left:?} {code} {right:?}");
    }
}
