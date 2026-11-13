//! Tests of the exported `int` arithmetic, on inline words and across the inline boundary.

use super::{
    typy_rt_int_absolute, typy_rt_int_add, typy_rt_int_compare, typy_rt_int_floor_divide,
    typy_rt_int_modulo, typy_rt_int_multiply, typy_rt_int_negate, typy_rt_int_power,
    typy_rt_int_subtract,
};
use crate::int::value::Integer;
use crate::word::inline;

fn word(value: i128) -> i64 {
    Integer::from_i128(value).into_word()
}

fn value(word: i64) -> i128 {
    // SAFETY: every word in these tests is an `int` word made by `Integer::into_word`.
    unsafe { Integer::from_word(word) }.to_i128().unwrap()
}

const LIMIT: i128 = 1 << 62;
const SAMPLES: [i128; 11] = [
    0,
    1,
    -1,
    5,
    -5,
    LIMIT - 1,
    -LIMIT,
    LIMIT,
    -LIMIT - 1,
    1 << 70,
    -(1 << 90),
];

#[test]
fn addition_subtraction_and_multiplication_cross_the_inline_boundary() {
    for a in SAMPLES {
        for b in SAMPLES {
            // SAFETY: the words are live `int` words.
            unsafe {
                assert_eq!(value(typy_rt_int_add(word(a), word(b))), a + b);
                assert_eq!(value(typy_rt_int_subtract(word(a), word(b))), a - b);
                if let Some(product) = a.checked_mul(b) {
                    assert_eq!(value(typy_rt_int_multiply(word(a), word(b))), product);
                }
            }
        }
    }
}

#[test]
fn floor_division_and_modulo_follow_python_signs() {
    let cases = [
        (7, 2, 3, 1),
        (-7, 2, -4, 1),
        (7, -2, -4, -1),
        (-7, -2, 3, -1),
        (6, 3, 2, 0),
        (-LIMIT, -1, LIMIT, 0),
        (1 << 70, 3, (1 << 70) / 3, (1 << 70) % 3),
    ];
    for (a, b, quotient, remainder) in cases {
        // SAFETY: the words are live `int` words.
        unsafe {
            let q = typy_rt_int_floor_divide(word(a), word(b));
            let r = typy_rt_int_modulo(word(a), word(b));
            assert!(q.exception.is_null() && r.exception.is_null());
            assert_eq!(
                (value(q.value), value(r.value)),
                (quotient, remainder),
                "{a} divmod {b}"
            );
        }
    }
}

#[test]
fn division_by_zero_raises_with_cpython_messages() {
    // SAFETY: the words are live `int` words, and the exceptions are live.
    unsafe {
        let q = typy_rt_int_floor_divide(word(1), word(0));
        assert_eq!(
            (*q.exception).kind(),
            crate::ExceptionKind::ZeroDivisionError
        );
        let r = typy_rt_int_modulo(word(1), word(0));
        assert!(!r.exception.is_null());
    }
}

#[test]
fn negation_absolute_power_and_comparison() {
    // SAFETY: the words are live `int` words.
    unsafe {
        assert_eq!(value(typy_rt_int_negate(word(-LIMIT))), LIMIT);
        assert_eq!(value(typy_rt_int_negate(word(1 << 70))), -(1 << 70));
        assert_eq!(value(typy_rt_int_absolute(word(-LIMIT))), LIMIT);
        assert_eq!(value(typy_rt_int_absolute(word(-(1 << 70)))), 1 << 70);
        assert_eq!(value(typy_rt_int_power(word(2), word(100)).value), 1 << 100);
        assert_eq!(value(typy_rt_int_power(word(-3), word(3)).value), -27);
        assert_eq!(value(typy_rt_int_power(word(7), word(0)).value), 1);
        assert!(typy_rt_int_compare(
            word(1 << 70),
            word(5),
            inline(4).unwrap()
        ));
        assert!(typy_rt_int_compare(
            word(-(1 << 70)),
            word(-5),
            inline(2).unwrap()
        ));
        assert!(typy_rt_int_compare(word(3), word(3), inline(0).unwrap()));
        assert!(typy_rt_int_compare(
            word(5),
            word(1 << 70),
            inline(2).unwrap()
        ));
        assert_eq!(value(typy_rt_int_absolute(word(1 << 70))), 1 << 70);
    }
}

#[test]
fn a_negative_exponent_raises_value_error() {
    // SAFETY: the words are live `int` words, and the exception is live.
    unsafe {
        let result = typy_rt_int_power(word(2), word(-1));
        assert!(!result.exception.is_null());
        assert_eq!((*result.exception).kind(), crate::ExceptionKind::ValueError);
    }
}
