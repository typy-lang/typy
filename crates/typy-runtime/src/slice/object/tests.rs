//! Tests of `slice` as a value.

use super::{
    SliceObject, slice_equal, slice_hash, slice_repr, typy_rt_slice_bound, typy_rt_slice_index,
    typy_rt_slice_new,
};
use crate::word::{inline, inline_value};

fn word(value: i64) -> i64 {
    inline(value).unwrap()
}

fn shown(slice: *mut SliceObject) -> String {
    // SAFETY: the slice is live, and the text is new and released here.
    unsafe {
        let text = slice_repr(slice.cast()).value;
        let shown = (0..crate::string::length(text))
            .map(|index| char::from_u32(crate::string::code_point(text, index)).unwrap())
            .collect();
        crate::refcount::typy_rt_decrement(text.cast());
        shown
    }
}

#[test]
fn a_slice_is_written_compared_and_hashed_as_cpython_does() {
    // SAFETY: the words are inline, and every slice made here is released here.
    unsafe {
        let one = typy_rt_slice_new(word(1), word(2), 0);
        let same = typy_rt_slice_new(word(1), word(2), 0);
        let other = typy_rt_slice_new(0, word(5), 0);
        assert_eq!(shown(one), "slice(1, 2, None)");
        assert!(slice_equal(one.cast(), same.cast()));
        assert!(!slice_equal(one.cast(), other.cast()));
        // `hash(slice(None, 5, None))` in CPython 3.12 to 3.15.
        assert_eq!(slice_hash(other.cast()), 1_765_991_802_173_826_621);
        // These bounds bring the sum to -1, which CPython gives as 1546275796.
        let summed = typy_rt_slice_new(word(6), word(2), word(737_508_419_540_398_855));
        assert_eq!(slice_hash(summed.cast()), 1_546_275_796);
        crate::refcount::typy_rt_decrement(summed.cast());
        assert_eq!(inline_value(typy_rt_slice_bound(one, word(1))), 2);
        assert_eq!(typy_rt_slice_bound(one, word(2)), 0);
        for slice in [one, same, other] {
            crate::refcount::typy_rt_decrement(slice.cast());
        }
    }
}

#[test]
fn indices_adjust_the_bounds_to_a_length_and_refuse_a_bad_one() {
    // SAFETY: the words are inline, and every object made here is released here.
    unsafe {
        let backwards = typy_rt_slice_new(word(-1), 0, word(-2));
        let parts: Vec<i64> = (0..3)
            .map(|part| inline_value(typy_rt_slice_index(backwards, word(5), word(part)).value))
            .collect();
        assert_eq!(parts, [4, -1, -2]);
        let error = typy_rt_slice_index(backwards, word(-1), word(0)).exception;
        assert!(!error.is_null());
        crate::refcount::typy_rt_decrement(error.cast());
        let zero = typy_rt_slice_new(0, 0, word(0));
        let error = typy_rt_slice_index(zero, word(5), word(0)).exception;
        assert!(!error.is_null());
        crate::refcount::typy_rt_decrement(error.cast());
        for slice in [backwards, zero] {
            crate::refcount::typy_rt_decrement(slice.cast());
        }
    }
}

#[test]
fn indices_give_a_missing_step_as_one_and_refuse_a_large_negative_length() {
    let text = crate::string::from_str(&format!("-{}", "9".repeat(30)));
    // SAFETY: the words are live, and every object made here is released here; the base
    // is the `int` word of 10.
    unsafe {
        let negative = crate::int::typy_rt_int_from_str(text, word(10)).value;
        crate::refcount::typy_rt_decrement(text.cast());
        let forwards = typy_rt_slice_new(word(1), word(3), 0);
        let parts: Vec<i64> = (0..3)
            .map(|part| inline_value(typy_rt_slice_index(forwards, word(5), word(part)).value))
            .collect();
        assert_eq!(parts, [1, 3, 1]);
        let error = typy_rt_slice_index(forwards, negative, word(0)).exception;
        assert_eq!((*error).class().name(), "ValueError");
        crate::refcount::typy_rt_decrement(error.cast());
        crate::refcount::typy_rt_int_decrement(negative);
        crate::refcount::typy_rt_decrement(forwards.cast());
    }
}

/// Returns the decimal text of an `int` word, which is released.
fn decimal(value: i64) -> String {
    // SAFETY: the word is live; it and the text are released here.
    unsafe {
        let text = crate::int::typy_rt_int_to_str(value).value;
        crate::refcount::typy_rt_int_decrement(value);
        let shown = (0..crate::string::length(text))
            .map(|index| char::from_u32(crate::string::code_point(text, index)).unwrap())
            .collect();
        crate::refcount::typy_rt_decrement(text.cast());
        shown
    }
}

/// Returns the `int` word of the decimal `text`, which the caller releases.
fn number(text: &str) -> i64 {
    let string = crate::string::from_str(text);
    // SAFETY: the text is live and released here; the base is the `int` word of 10.
    unsafe {
        let value = crate::int::typy_rt_int_from_str(string, word(10)).value;
        crate::refcount::typy_rt_decrement(string.cast());
        value
    }
}

#[test]
#[cfg_attr(
    miri,
    ignore = "building a 4300-digit integer takes hours under Miri; the same code runs in the other tests"
)]
fn a_bound_of_more_than_4300_digits_is_not_written() {
    let mut big = crate::int::value::Integer::from_i128(1);
    let ten = crate::int::value::Integer::from_i128(10);
    for _ in 0..4300 {
        big = big.multiply(&ten);
    }
    let bound = big.into_word();
    // SAFETY: the slice takes its own reference to the word; the word, the slice and the
    // exception are released here.
    unsafe {
        let slice = typy_rt_slice_new(bound, 0, 0);
        crate::refcount::typy_rt_int_decrement(bound);
        let written = slice_repr(slice.cast());
        assert!(written.value.is_null());
        let error = written.exception;
        assert_eq!((*error).class().name(), "ValueError");
        crate::refcount::typy_rt_decrement(error.cast());
        crate::refcount::typy_rt_decrement(slice.cast());
    }
}

#[test]
fn indices_hold_each_bound_inside_a_length_past_an_inline_int() {
    let length = "9".repeat(30);
    let below = format!("-{}", "9".repeat(31));
    // The bounds of each slice, with the start, the stop and the step CPython gives for
    // `length`.
    let cases = [
        ((Some("1"), Some("3"), None), ("1", "3", "1")),
        (
            (None, None, Some("-1")),
            ("999999999999999999999999999998", "-1", "-1"),
        ),
        (
            (Some("-5"), None, None),
            (
                "999999999999999999999999999994",
                "999999999999999999999999999999",
                "1",
            ),
        ),
        (
            (Some(length.as_str()), Some(below.as_str()), Some("-2")),
            ("999999999999999999999999999998", "-1", "-2"),
        ),
    ];
    for ((start, stop, step), (from, to, by)) in cases {
        let bound = |text: Option<&str>| text.map_or(0, number);
        let given = number(&length);
        // SAFETY: every word is live, and every object made here is released here.
        unsafe {
            let slice = typy_rt_slice_new(bound(start), bound(stop), bound(step));
            let found = (
                decimal(typy_rt_slice_index(slice, given, word(0)).value),
                decimal(typy_rt_slice_index(slice, given, word(1)).value),
                decimal(typy_rt_slice_index(slice, given, word(2)).value),
            );
            assert_eq!(found, (from.to_owned(), to.to_owned(), by.to_owned()));
            crate::refcount::typy_rt_int_decrement(given);
            crate::refcount::typy_rt_decrement(slice.cast());
        }
    }
    let given = number(&length);
    // SAFETY: as above.
    unsafe {
        let still = typy_rt_slice_new(0, 0, word(0));
        let error = typy_rt_slice_index(still, given, word(0)).exception;
        assert_eq!((*error).class().name(), "ValueError");
        crate::refcount::typy_rt_decrement(error.cast());
        crate::refcount::typy_rt_int_decrement(given);
        crate::refcount::typy_rt_decrement(still.cast());
    }
}
