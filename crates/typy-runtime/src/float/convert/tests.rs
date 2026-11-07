//! Tests of exact conversion and comparison between `int` and `float`.

use super::{
    typy_rt_compare_int_float, typy_rt_float_to_int, typy_rt_int_to_float, typy_rt_int_true_divide,
};
use crate::Raising;
use crate::int::natural::shift_left;
use crate::int::value::Integer;
use crate::word::inline;

fn word(value: i128) -> i64 {
    Integer::from_i128(value).into_word()
}

fn power_of_two(bits: u64) -> i64 {
    Integer {
        negative: false,
        magnitude: shift_left(&[1], bits),
    }
    .into_word()
}

fn message<T>(result: &Raising<T>) -> String {
    assert!(!result.exception.is_null(), "no exception");
    // SAFETY: the exception and its message are live.
    unsafe {
        let text = (*result.exception).message;
        (0..crate::string::length(text))
            .map(|index| char::from_u32(crate::string::code_point(text, index)).unwrap())
            .collect()
    }
}

#[test]
fn integers_convert_with_one_rounding() {
    #[allow(
        clippy::cast_precision_loss,
        reason = "the conversion under test rounds"
    )]
    for value in [
        0,
        5,
        -5,
        1 << 53,
        (1 << 53) + 1,
        -(1 << 62),
        1 << 62,
        (1 << 100) + 1,
        -(1 << 120),
    ] {
        // SAFETY: the word is a live `int` word.
        let result = unsafe { typy_rt_int_to_float(word(value)) };
        assert_eq!(result.value, value as f64, "{value}");
    }
    // SAFETY: the word is a live `int` word.
    let result = unsafe { typy_rt_int_to_float(power_of_two(1024)) };
    assert_eq!(message(&result), "int too large to convert to float");
}

#[test]
fn integer_division_rounds_once_and_keeps_the_sign() {
    let cases: [(i128, i128, f64); 6] = [
        (7, 2, 3.5),
        (1, 3, 1.0 / 3.0),
        (0, -5, -0.0),
        (-1, 1 << 100, -7.888_609_052_210_118e-31),
        ((1 << 100) + 1, 1 << 100, 1.0),
        (10_i128.pow(30), -(10_i128.pow(28)), -100.0),
    ];
    for (left, right, expected) in cases {
        // SAFETY: the words are live `int` words.
        let result = unsafe { typy_rt_int_true_divide(word(left), word(right)) };
        assert!(result.exception.is_null());
        assert_eq!(
            result.value.to_bits(),
            expected.to_bits(),
            "{left} / {right}"
        );
    }
    // SAFETY: the words are live `int` words.
    unsafe {
        assert_eq!(
            message(&typy_rt_int_true_divide(word(1), word(0))),
            "division by zero"
        );
        assert_eq!(
            message(&typy_rt_int_true_divide(power_of_two(1100), word(3))),
            "integer division result too large for a float"
        );
        assert_eq!(
            typy_rt_int_true_divide(word(1), power_of_two(1200)).value,
            0.0
        );
    }
}

#[test]
fn floats_truncate_toward_zero() {
    for (value, expected) in [
        (2.9, 2),
        (-2.9, -2),
        (-0.0, 0),
        (1e20, 100_000_000_000_000_000_000),
        (-1.5e30, -1_499_999_999_999_999_889_089_448_902_656),
    ] {
        let result = typy_rt_float_to_int(value);
        // SAFETY: the word is a live `int` word.
        let integer = unsafe { Integer::from_word(result.value) };
        assert_eq!(integer.to_i128(), Some(expected), "{value}");
    }
    assert_eq!(
        message(&typy_rt_float_to_int(f64::NAN)),
        "cannot convert float NaN to integer"
    );
    assert_eq!(
        message(&typy_rt_float_to_int(f64::NEG_INFINITY)),
        "cannot convert float infinity to integer"
    );
}

#[test]
fn comparisons_use_the_exact_values() {
    let less = inline(2).unwrap();
    let equal = inline(0).unwrap();
    let not_equal = inline(1).unwrap();
    let big = (1_i128 << 53) + 1;
    #[allow(
        clippy::cast_precision_loss,
        reason = "the rounding is the point of the test"
    )]
    let rounded = big as f64;
    let cases: [(i128, f64, i64, bool); 12] = [
        (1, 2.5, less, true),
        (3, 2.5, less, false),
        (-5, -4.5, less, true),
        (-4, -4.5, less, false),
        (big, rounded, equal, false),
        (big - 1, rounded, equal, true),
        (1 << 100, 1.267_650_600_228_229_4e30, equal, true),
        ((1 << 100) + 1, 1.267_650_600_228_229_4e30, less, false),
        (1 << 100, f64::INFINITY, less, true),
        (-(1 << 100), f64::NEG_INFINITY, less, false),
        (0, f64::NAN, equal, false),
        (0, f64::NAN, not_equal, true),
    ];
    for (left, right, comparison, expected) in cases {
        // SAFETY: the words are live `int` words.
        let result = unsafe { typy_rt_compare_int_float(word(left), right, comparison) };
        assert_eq!(result, expected, "{left} vs {right}");
    }
    // SAFETY: the words are live `int` words.
    unsafe {
        assert!(typy_rt_compare_int_float(
            word(1 << 62),
            4.6e18,
            inline(4).unwrap()
        ));
        assert!(typy_rt_compare_int_float(
            word(1 << 62),
            4.611_686_018_427_388e18,
            equal
        ));
        assert!(typy_rt_compare_int_float(
            word((1 << 62) + 1),
            4.611_686_018_427_388e18,
            inline(4).unwrap()
        ));
    }
}
