//! Tests of `repr(float)` against CPython's output and against Rust's shortest digits.

use super::{shortest, typy_rt_float_to_str};
use crate::string::{code_point, length};

fn repr(value: f64) -> String {
    let text = typy_rt_float_to_str(value);
    // SAFETY: the string is live, and every index is in bounds.
    (0..unsafe { length(text) })
        .map(|index| char::from_u32(unsafe { code_point(text, index) }).unwrap())
        .collect()
}

#[test]
fn floats_are_written_as_cpython_writes_them() {
    let cases: [(f64, &str); 27] = [
        (2_138_389_708_628_431.2, "2138389708628431.2"),
        (1e16, "1e+16"),
        (1e15, "1000000000000000.0"),
        (123_456_789_012_345_680.0, "1.2345678901234568e+17"),
        (0.1, "0.1"),
        (1e-5, "1e-05"),
        (0.0001, "0.0001"),
        (-0.0, "-0.0"),
        (0.0, "0.0"),
        (1.5e300, "1.5e+300"),
        (5e-324, "5e-324"),
        (f64::INFINITY, "inf"),
        (f64::NEG_INFINITY, "-inf"),
        (f64::NAN, "nan"),
        (-f64::NAN, "nan"),
        (2.5, "2.5"),
        (100.0, "100.0"),
        (1e22, "1e+22"),
        (f64::MAX, "1.7976931348623157e+308"),
        (0.1 + 0.2, "0.30000000000000004"),
        (1.0 / 3.0, "0.3333333333333333"),
        (-1.5, "-1.5"),
        (123.456, "123.456"),
        (9.223_372_036_854_776e18, "9.223372036854776e+18"),
        (1e-7, "1e-07"),
        (2.225_073_858_507_201_4e-308, "2.2250738585072014e-308"),
        (1e100, "1e+100"),
    ];
    for (value, expected) in cases {
        assert_eq!(repr(value), expected);
    }
}

/// Returns Rust's shortest digits of `value` and the position of the decimal point.
fn rust_shortest(value: f64) -> (String, i64) {
    let text = format!("{value:e}");
    let (mantissa, exponent) = text.split_once('e').unwrap();
    (
        mantissa.replace('.', ""),
        exponent.parse::<i64>().unwrap() + 1,
    )
}

/// Checks that the digits of `value` read back, are as short as Rust's, and differ from Rust's
/// only at an exact tie, which CPython rounds to the even digit where Rust rounds up.
fn check(value: f64) {
    let decimal = shortest(value);
    let digits: String = decimal
        .digits()
        .iter()
        .map(|digit| char::from(b'0' + digit))
        .collect();
    let count = i64::try_from(digits.len()).unwrap();
    let read_back: f64 = format!("{digits}e{}", decimal.point - count)
        .parse()
        .unwrap();
    assert_eq!(read_back.to_bits(), value.to_bits(), "{value:e}");
    let (expected, point) = rust_shortest(value);
    assert_eq!(
        (digits.len(), decimal.point),
        (expected.len(), point),
        "{value:e}"
    );
    if digits != expected {
        let last = *decimal.digits().last().unwrap();
        let rust_last = expected.as_bytes()[expected.len() - 1] - b'0';
        assert!(
            last.is_multiple_of(2) && rust_last == last + 1,
            "{value:e}: {digits} {expected}"
        );
    }
}

#[test]
fn digits_are_the_shortest_that_read_back() {
    let mut state = 0x2545_f491_4f6c_dd1d_u64;
    for _ in 0..20_000 {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        let value = f64::from_bits(state >> 1);
        if value.is_finite() && value != 0.0 {
            check(value);
        }
    }
    for value in [
        5e-324,
        1e-323,
        f64::MIN_POSITIVE,
        2.0,
        1024.0,
        f64::MAX,
        9_007_199_254_740_993.0,
        2_138_389_708_628_431.2,
    ] {
        check(value);
    }
}
