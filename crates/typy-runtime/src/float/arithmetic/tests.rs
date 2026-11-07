//! Tests of `float` arithmetic against results printed by CPython 3.12 to 3.15.

use super::{
    typy_rt_float_absolute, typy_rt_float_divide, typy_rt_float_floor_divide, typy_rt_float_modulo,
    typy_rt_float_power,
};
use crate::Raising;

const INF: f64 = f64::INFINITY;

fn message(result: &Raising<f64>) -> String {
    assert!(!result.exception.is_null(), "no exception");
    // SAFETY: the exception and its message are live.
    unsafe {
        let text = (*result.exception).message;
        (0..crate::string::length(text))
            .map(|index| char::from_u32(crate::string::code_point(text, index)).unwrap())
            .collect()
    }
}

fn same(result: &Raising<f64>, expected: f64) -> bool {
    result.exception.is_null()
        && (result.value.to_bits() == expected.to_bits()
            || (result.value.is_nan() && expected.is_nan()))
}

#[test]
fn division_by_zero_raises_for_every_division() {
    for operation in [
        typy_rt_float_divide,
        typy_rt_float_floor_divide,
        typy_rt_float_modulo,
    ] {
        for zero in [0.0, -0.0] {
            assert_eq!(message(&operation(1.0, zero)), "division by zero");
        }
    }
}

#[test]
fn floor_division_and_modulo_follow_the_sign_of_the_divisor() {
    let cases: [(f64, f64, f64, f64); 9] = [
        (-7.5, 2.0, -4.0, 0.5),
        (7.5, -2.0, -4.0, -0.5),
        (7.5, 2.0, 3.0, 1.5),
        (-0.0, 5.0, -0.0, 0.0),
        (0.0, -5.0, -0.0, -0.0),
        (INF, 3.0, f64::NAN, f64::NAN),
        (3.0, INF, 0.0, 3.0),
        (-3.0, INF, -1.0, INF),
        (0.1, 0.01, 10.0, 3.469_446_951_953_614e-18),
    ];
    for (left, right, quotient, remainder) in cases {
        assert!(
            same(&typy_rt_float_floor_divide(left, right), quotient),
            "{left} // {right}"
        );
        assert!(
            same(&typy_rt_float_modulo(left, right), remainder),
            "{left} % {right}"
        );
    }
    assert!(same(&typy_rt_float_divide(7.0, 2.0), 3.5));
}

#[test]
fn powers_follow_the_special_cases_of_cpython() {
    let cases: [(f64, f64, f64); 14] = [
        (2.0, 10.0, 1024.0),
        (-2.0, 2.0, 4.0),
        (-2.0, 3.0, -8.0),
        (-1.0, 3.0, -1.0),
        (f64::NAN, 0.0, 1.0),
        (1.0, f64::NAN, 1.0),
        (2.0, f64::NAN, f64::NAN),
        (f64::NAN, 2.0, f64::NAN),
        (0.5, INF, 0.0),
        (2.0, INF, INF),
        (-1.0, -INF, 1.0),
        (-INF, -3.0, -0.0),
        (-INF, 3.0, -INF),
        (-0.0, 3.0, -0.0),
    ];
    for (base, exponent, expected) in cases {
        assert!(
            same(&typy_rt_float_power(base, exponent), expected),
            "{base} ** {exponent}"
        );
    }
    assert!(same(&typy_rt_float_power(-INF, 2.0), INF));
    assert!(same(&typy_rt_float_power(-INF, -2.0), 0.0));
    assert!(same(&typy_rt_float_power(-0.0, 2.0), 0.0));
}

#[test]
fn powers_out_of_range_raise() {
    assert_eq!(
        message(&typy_rt_float_power(0.0, -1.0)),
        "zero to a negative power"
    );
    assert_eq!(
        message(&typy_rt_float_power(10.0, 400.0)),
        "(34, 'Numerical result out of range')"
    );
    assert_eq!(
        message(&typy_rt_float_power(-8.0, 0.5)),
        "math domain error"
    );
}

#[test]
fn absolute_values_clear_the_sign() {
    assert_eq!(typy_rt_float_absolute(-0.0).to_bits(), 0.0_f64.to_bits());
    assert_eq!(typy_rt_float_absolute(-2.5), 2.5);
}
