//! Tests of `round`, against results CPython 3.15 gives.

use super::{typy_rt_float_round, typy_rt_float_round_digits, typy_rt_int_round_digits};
use crate::int::value::Integer;
use crate::word::{inline, inline_value};

fn word(value: i64) -> i64 {
    inline(value).unwrap()
}

#[test]
#[cfg_attr(miri, ignore = "Miri cannot call the C maths library's `floor`")]
fn floats_round_to_the_even_integer() {
    let cases = [
        (0.5, 0),
        (1.5, 2),
        (2.5, 2),
        (-0.5, 0),
        (-2.5, -2),
        (-1.5, -2),
        (0.499_999_999_999_999_94, 0),
        (4_503_599_627_370_497.0, 4_503_599_627_370_497),
    ];
    for (value, expected) in cases {
        let rounded = typy_rt_float_round(value);
        assert!(rounded.exception.is_null());
        assert_eq!(inline_value(rounded.value), expected, "{value}");
    }
    assert!(!typy_rt_float_round(f64::NAN).exception.is_null());
    assert!(!typy_rt_float_round(f64::NEG_INFINITY).exception.is_null());
}

#[test]
fn floats_round_at_a_number_of_digits_as_cpython_rounds_them() {
    let cases = [
        (2.675, 2, 2.67),
        (1.005, 2, 1.0),
        (0.125, 2, 0.12),
        (0.375, 2, 0.38),
        (123.456, -1, 120.0),
        (-123.456, 1, -123.5),
        (2.5, 0, 2.0),
        (-0.4, 0, -0.0),
        (5e-324, 323, 0.0),
        (5e-324, 324, 5e-324),
        (1e300, -300, 1e300),
        (99.95, 1, 100.0),
        (9.995, 2, 9.99),
        (0.049_999_999_999_999_996, 1, 0.0),
        (4_503_599_627_370_497.0, -1, 4_503_599_627_370_500.0),
        (1e16, -16, 1e16),
        (2.225_073_858_507_201_4e-308, 310, 2.23e-308),
        (-2.5, -400, -0.0),
        (f64::INFINITY, 2, f64::INFINITY),
    ];
    for (value, digits, expected) in cases {
        // SAFETY: the digits are an inline word.
        let rounded = unsafe { typy_rt_float_round_digits(value, word(digits)) };
        assert!(rounded.exception.is_null(), "{value} {digits}");
        assert_eq!(
            rounded.value.to_bits(),
            f64::to_bits(expected),
            "{value} {digits}"
        );
    }
    let huge = Integer::from_i128(1 << 70).into_word();
    let tiny = Integer::from_i128(-(1 << 70)).into_word();
    // SAFETY: the digits are live words.
    unsafe {
        assert_eq!(typy_rt_float_round_digits(0.1, huge).value, 0.1);
        assert_eq!(
            typy_rt_float_round_digits(-0.1, tiny).value.to_bits(),
            (-0.0_f64).to_bits()
        );
        assert!(typy_rt_float_round_digits(f64::NAN, word(1)).value.is_nan());
        assert!(
            !typy_rt_float_round_digits(f64::MAX, word(-308))
                .exception
                .is_null()
        );
    }
}

#[test]
fn integers_round_to_the_even_multiple_of_a_power_of_ten() {
    let cases: [(i128, i64, i128); 11] = [
        (15, -1, 20),
        (25, -1, 20),
        (-25, -1, -20),
        (149, -2, 100),
        (150, -2, 200),
        (250, -2, 200),
        (-150, -2, -200),
        (5, -1, 0),
        (4, -1, 0),
        (
            1_500_000_000_000_000_000_000_000_000_000,
            -30,
            2_000_000_000_000_000_000_000_000_000_000,
        ),
        (123, 2, 123),
    ];
    for (value, digits, expected) in cases {
        let value_word = Integer::from_i128(value).into_word();
        // SAFETY: the words are live.
        let rounded = unsafe { typy_rt_int_round_digits(value_word, word(digits)) };
        // SAFETY: the result is a live word.
        let rounded = unsafe { Integer::from_word(rounded) }.to_i128();
        assert_eq!(rounded, Some(expected), "{value} {digits}");
    }
    let tiny = Integer::from_i128(-(1 << 70)).into_word();
    // SAFETY: the words are live.
    assert_eq!(unsafe { typy_rt_int_round_digits(word(99), tiny) }, word(0));
}
