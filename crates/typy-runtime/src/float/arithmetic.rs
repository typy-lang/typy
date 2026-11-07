//! `float` division, floor division, modulo, power and `abs`, as CPython computes them (EXP-03).

#![allow(
    clippy::float_cmp,
    reason = "CPython's special cases test for exact values such as 1.0 and integral exponents"
)]

use crate::Raising;
use crate::exception::{ExceptionKind, raise};
use crate::int::ZERO_DIVISION;
use crate::libm::{absolute, copy_sign, floor, fmod, pow};

/// CPython reports a result too large for a double with the text of `ERANGE`.
pub(crate) const OUT_OF_RANGE: &str = "(34, 'Numerical result out of range')";

fn zero_division() -> Raising<f64> {
    Raising::error(0.0, raise(ExceptionKind::ZeroDivisionError, ZERO_DIVISION))
}

/// Returns `left / right`.
#[unsafe(no_mangle)]
pub extern "C" fn typy_rt_float_divide(left: f64, right: f64) -> Raising<f64> {
    if right == 0.0 {
        return zero_division();
    }
    Raising::ok(left / right)
}

/// Returns `divmod(left, right)` for a non-zero `right`, as CPython's `float_divmod` does: the
/// remainder takes the sign of `right`, and the quotient is rounded to the nearest integer to
/// correct the error of `fmod`.
fn divide_and_modulo(left: f64, right: f64) -> (f64, f64) {
    let mut modulo = fmod(left, right);
    let mut quotient = (left - modulo) / right;
    if modulo == 0.0 {
        modulo = copy_sign(0.0, right);
    } else if (right < 0.0) != (modulo < 0.0) {
        modulo += right;
        quotient -= 1.0;
    }
    let floor_quotient = if quotient == 0.0 {
        copy_sign(0.0, left / right)
    } else {
        let whole = floor(quotient);
        if quotient - whole > 0.5 {
            whole + 1.0
        } else {
            whole
        }
    };
    (floor_quotient, modulo)
}

/// Returns `left // right`.
#[unsafe(no_mangle)]
pub extern "C" fn typy_rt_float_floor_divide(left: f64, right: f64) -> Raising<f64> {
    if right == 0.0 {
        return zero_division();
    }
    Raising::ok(divide_and_modulo(left, right).0)
}

/// Returns `left % right`, with the sign of `right`.
#[unsafe(no_mangle)]
pub extern "C" fn typy_rt_float_modulo(left: f64, right: f64) -> Raising<f64> {
    if right == 0.0 {
        return zero_division();
    }
    Raising::ok(divide_and_modulo(left, right).1)
}

fn is_odd_integer(value: f64) -> bool {
    fmod(absolute(value), 2.0) == 1.0
}

/// Returns `base ** exponent` with the special cases of CPython's `float_pow`.
#[unsafe(no_mangle)]
pub extern "C" fn typy_rt_float_power(base: f64, exponent: f64) -> Raising<f64> {
    if exponent == 0.0 {
        return Raising::ok(1.0);
    }
    if base.is_nan() {
        return Raising::ok(base);
    }
    if exponent.is_nan() {
        return Raising::ok(if base == 1.0 { 1.0 } else { exponent });
    }
    if exponent.is_infinite() {
        let size = absolute(base);
        let result = if size == 1.0 {
            1.0
        } else if (exponent > 0.0) == (size > 1.0) {
            f64::INFINITY
        } else {
            0.0
        };
        return Raising::ok(result);
    }
    let odd = is_odd_integer(exponent);
    if base.is_infinite() {
        let result = match (exponent > 0.0, odd) {
            (true, true) => base,
            (true, false) => absolute(base),
            (false, true) => copy_sign(0.0, base),
            (false, false) => 0.0,
        };
        return Raising::ok(result);
    }
    if base == 0.0 {
        if exponent < 0.0 {
            return Raising::error(
                0.0,
                raise(ExceptionKind::ZeroDivisionError, "zero to a negative power"),
            );
        }
        return Raising::ok(if odd { base } else { 0.0 });
    }
    let negate = base < 0.0;
    if negate && floor(exponent) != exponent {
        // CPython returns a `complex` here; EXP-04 rejects every program that could reach it.
        return Raising::error(0.0, raise(ExceptionKind::ValueError, "math domain error"));
    }
    let size = absolute(base);
    let negate = negate && odd;
    if size == 1.0 {
        return Raising::ok(if negate { -1.0 } else { 1.0 });
    }
    let result = pow(size, exponent);
    if result.is_infinite() {
        return Raising::error(0.0, raise(ExceptionKind::OverflowError, OUT_OF_RANGE));
    }
    Raising::ok(if negate { -result } else { result })
}

/// Returns `abs(value)`.
#[unsafe(no_mangle)]
pub extern "C" fn typy_rt_float_absolute(value: f64) -> f64 {
    absolute(value)
}

#[cfg(test)]
mod tests;
