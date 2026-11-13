//! Tests of `range` as a value, against CPython's results.

use super::{
    RangeObject, typy_rt_range_bound, typy_rt_range_contains, typy_rt_range_equal,
    typy_rt_range_get, typy_rt_range_length, typy_rt_range_new, typy_rt_range_slice,
};
use crate::header::ObjectHeader;
use crate::int::value::Integer;
use crate::string::{StrObject, code_point, length};
use crate::word::{inline, inline_value};

fn word(value: i64) -> i64 {
    inline(value).unwrap()
}

/// Returns `range(start, stop, step)`.
fn range(start: i64, stop: i64, step: i64) -> *mut RangeObject {
    // SAFETY: the words are inline.
    let made = unsafe { typy_rt_range_new(word(start), word(stop), word(step)) };
    assert!(made.exception.is_null());
    made.value
}

fn release<T>(object: *mut T) {
    // SAFETY: the test holds a reference.
    unsafe { crate::refcount::typy_rt_decrement(object.cast()) };
}

fn text(value: *mut StrObject) -> String {
    // SAFETY: the string is live and owned here.
    unsafe {
        let shown = (0..length(value))
            .map(|index| char::from_u32(code_point(value, index)).unwrap())
            .collect();
        crate::refcount::typy_rt_decrement(value.cast());
        shown
    }
}

/// Returns `repr(range)`.
fn repr(range: *mut RangeObject) -> String {
    // SAFETY: the range is live.
    text(unsafe { super::range_repr(range.cast()) }.value)
}

/// Returns `len(range)`.
fn len(range: *mut RangeObject) -> i64 {
    // SAFETY: the range is live.
    inline_value(unsafe { typy_rt_range_length(range) }.value)
}

#[test]
fn a_range_has_cpython_s_length_items_and_text() {
    let cases: [(i64, i64, i64, i64, &str); 5] = [
        (0, 5, 1, 5, "range(0, 5)"),
        (0, 10, 3, 4, "range(0, 10, 3)"),
        (10, 0, -3, 4, "range(10, 0, -3)"),
        (5, 5, 1, 0, "range(5, 5)"),
        (0, 5, -1, 0, "range(0, 5, -1)"),
    ];
    for (start, stop, step, length, shown) in cases {
        let made = range(start, stop, step);
        assert_eq!(len(made), length, "{shown}");
        assert_eq!(repr(made), shown);
        release(made);
    }
}

#[test]
fn a_zero_step_raises_value_error() {
    // SAFETY: the words are inline.
    let made = unsafe { typy_rt_range_new(word(0), word(1), word(0)) };
    assert_eq!(
        // SAFETY: the exception is live.
        unsafe { (*made.exception).kind() },
        crate::ExceptionKind::ValueError
    );
    release(made.exception);
}

#[test]
fn indexing_counts_from_either_end_and_refuses_one_outside() {
    let made = range(10, 0, -3);
    for (index, value) in [(0, 10), (3, 1), (-1, 1), (-4, 10)] {
        // SAFETY: the range is live.
        let item = unsafe { typy_rt_range_get(made, word(index)) };
        assert_eq!(inline_value(item.value), value, "[{index}]");
    }
    for index in [4, -5] {
        // SAFETY: the range is live.
        let item = unsafe { typy_rt_range_get(made, word(index)) };
        assert_eq!(
            // SAFETY: the exception is live.
            unsafe { (*item.exception).kind() },
            crate::ExceptionKind::IndexError
        );
        release(item.exception);
    }
    release(made);
}

#[test]
fn a_slice_is_the_range_cpython_builds() {
    // The slice's start, stop, step and mask, and the range it gives.
    let cases: [(i64, i64, i64, i64, &str); 4] = [
        (1, 8, 3, 7, "range(1, 8, 3)"),
        (0, 0, -1, 4, "range(9, -1, -1)"),
        (1, 100, 0, 3, "range(1, 10)"),
        (0, 0, 2, 4, "range(0, 10, 2)"),
    ];
    let made = range(0, 10, 1);
    for (start, stop, step, mask, shown) in cases {
        // SAFETY: the range is live and the bounds are inline words.
        let sliced =
            unsafe { typy_rt_range_slice(made, word(start), word(stop), word(step), word(mask)) };
        assert_eq!(repr(sliced.value), shown);
        release(sliced.value);
    }
    release(made);
}

#[test]
fn membership_and_equality_follow_cpython() {
    let made = range(1, 10, 3);
    for (value, inside) in [(1, true), (7, true), (8, false), (10, false), (0, false)] {
        assert_eq!(
            // SAFETY: the range is live.
            unsafe { typy_rt_range_contains(made, word(value)) },
            inside,
            "{value}"
        );
    }
    let falling = range(9, 0, -4);
    for (value, inside) in [(9, true), (1, true), (5, true), (0, false)] {
        assert_eq!(
            // SAFETY: the range is live.
            unsafe { typy_rt_range_contains(falling, word(value)) },
            inside,
            "{value}"
        );
    }
    let pairs = [
        ((0, 0, 1), (5, 5, 2), true),
        ((1, 2, 1), (1, 9, 10), true),
        ((0, 3, 1), (0, 3, 2), false),
        ((0, 3, 1), (0, 3, 1), true),
        ((0, 3, 1), (1, 4, 1), false),
    ];
    for ((a, b, c), (d, e, f), equal) in pairs {
        let (left, right) = (range(a, b, c), range(d, e, f));
        // SAFETY: both ranges are live.
        assert_eq!(unsafe { typy_rt_range_equal(left, right) }, equal);
        // SAFETY: the type of each object says it is a range.
        let same = unsafe { super::range_hash(left.cast()) == super::range_hash(right.cast()) };
        assert_eq!(same, equal);
        release(left);
        release(right);
    }
    release(falling);
    release(made);
}

#[test]
fn bounds_and_truth_come_from_the_range() {
    let made = range(2, 9, 3);
    // SAFETY: the range is live; inline words hold no reference.
    unsafe {
        let bounds = [0, 1, 2].map(|which| inline_value(typy_rt_range_bound(made, word(which))));
        assert_eq!(bounds, [2, 9, 3]);
        assert!(super::range_is_true(made.cast()));
    }
    let empty = range(0, 0, 1);
    // SAFETY: the type says it is a range.
    assert!(!unsafe { super::range_is_true(empty.cast::<ObjectHeader>()) });
    release(empty);
    release(made);
}

#[test]
fn a_range_past_sys_maxsize_has_no_length() {
    let huge = Integer::from_i128(1 << 70).into_word();
    // SAFETY: the words are live.
    let made = unsafe { typy_rt_range_new(word(0), huge, word(1)) }.value;
    // SAFETY: the range is live.
    let length = unsafe { typy_rt_range_length(made) };
    assert_eq!(
        // SAFETY: the exception is live.
        unsafe { (*length.exception).kind() },
        crate::ExceptionKind::OverflowError
    );
    release(length.exception);
    // SAFETY: the range is live and the word is inline-free.
    unsafe {
        assert!(typy_rt_range_contains(made, word(5)));
        let text = super::range_repr(made.cast());
        assert_eq!(
            super::tests::text(text.value),
            "range(0, 1180591620717411303424)"
        );
        crate::refcount::typy_rt_int_decrement(huge);
    }
    release(made);
}
