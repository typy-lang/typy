//! Tests of `int` to and from `str`, including Python's parsing rules and the digit limit.

use super::{typy_rt_int_from_str, typy_rt_int_to_str};
use crate::int::value::Integer;
use crate::string::{code_point, from_str, length};
use crate::word::inline;

fn text(string: *const crate::StrObject) -> String {
    // SAFETY: the string is live and every index is in bounds.
    (0..unsafe { length(string) })
        .map(|index| char::from_u32(unsafe { code_point(string, index) }).unwrap())
        .collect()
}

fn word(value: i128) -> i64 {
    Integer::from_i128(value).into_word()
}

fn parse(source: &str, base: i64) -> Result<i128, String> {
    // SAFETY: the string is live, and the result's exception and value are live.
    unsafe {
        let result = typy_rt_int_from_str(from_str(source), inline(base).unwrap());
        if result.exception.is_null() {
            Ok(Integer::from_word(result.value).to_i128().unwrap())
        } else {
            Err(text((*result.exception).message))
        }
    }
}

#[test]
fn integers_are_written_in_decimal() {
    for value in [
        0,
        7,
        -7,
        (1 << 62) - 1,
        -(1 << 62),
        1 << 64,
        -(1 << 100),
        10_i128.pow(19),
        10_i128.pow(19) - 1,
        i128::MAX,
    ] {
        // SAFETY: the word is a live `int` word.
        let result = unsafe { typy_rt_int_to_str(word(value)) };
        assert_eq!(text(result.value), value.to_string());
    }
}

#[test]
#[cfg_attr(
    miri,
    ignore = "building a 4300-digit integer takes hours under Miri; the same code runs in the other tests"
)]
fn integers_of_more_than_4300_digits_are_refused() {
    let mut big = Integer::from_i128(1);
    let ten = Integer::from_i128(10);
    for _ in 0..4300 {
        big = big.multiply(&ten);
    }
    // SAFETY: the word is a live `int` word, and the exception is live.
    unsafe {
        let result = typy_rt_int_to_str(big.into_word());
        assert!(text((*result.exception).message).starts_with("Exceeds the limit (4300 digits)"));
    }
}

#[test]
fn text_is_parsed_with_python_rules() {
    let cases: [(&str, i64, i128); 21] = [
        ("\u{661}\u{662}", 10, 12),
        ("\u{663}", 16, 3),
        ("0x\u{661}\u{662}", 16, 18),
        ("\u{ff11}\u{ff12}", 10, 12),
        ("\u{661}_\u{662}", 10, 12),
        ("12", 10, 12),
        ("  -12\n", 10, -12),
        ("+7", 10, 7),
        ("1_000", 10, 1000),
        ("ff", 16, 255),
        ("0xff", 16, 255),
        ("0x_ff", 16, 255),
        ("0x10", 0, 16),
        ("0o17", 0, 15),
        ("0b101", 0, 5),
        ("010", 8, 8),
        ("00", 0, 0),
        ("0_0", 0, 0),
        ("z", 36, 35),
        ("-0", 10, 0),
        (
            "123456789012345678901234567890",
            10,
            123_456_789_012_345_678_901_234_567_890,
        ),
    ];
    for (source, base, expected) in cases {
        assert_eq!(parse(source, base), Ok(expected), "int({source:?}, {base})");
    }
}

#[test]
fn malformed_text_raises_with_the_quoted_input() {
    for (source, base) in [
        ("", 10),
        ("x", 10),
        ("1__0", 10),
        ("_1", 10),
        ("1_", 10),
        ("010", 0),
        ("0x", 16),
        ("12", 2),
        ("0x10", 10),
        (" ", 10),
        ("\u{663}", 2),
        ("\u{b2}", 10),
        ("\u{661}x", 10),
    ] {
        let message = parse(source, base).expect_err(source);
        assert!(
            message.starts_with(&format!("invalid literal for int() with base {base}: ")),
            "{message}"
        );
    }
    assert_eq!(
        parse("x y", 10),
        Err("invalid literal for int() with base 10: 'x y'".to_owned())
    );
    assert_eq!(
        parse("1", 1),
        Err("int() base must be >= 2 and <= 36, or 0".to_owned())
    );
    assert_eq!(
        parse("1", 37),
        Err("int() base must be >= 2 and <= 36, or 0".to_owned())
    );
}

#[test]
fn a_base_that_is_a_large_int_is_refused() {
    // SAFETY: the string is live, the base word points to a live large integer, and the
    // exception is live.
    unsafe {
        let result = typy_rt_int_from_str(from_str("10"), word(1 << 70));
        assert!(text((*result.exception).message).starts_with("int() base must be >= 2 and <= 36"));
    }
}

#[test]
fn text_of_more_than_4300_decimal_digits_is_refused() {
    let digits = "1".repeat(4301);
    let refused = parse(&digits, 10).unwrap_err();
    assert!(
        refused.starts_with("Exceeds the limit (4300 digits)"),
        "{refused}"
    );
}
