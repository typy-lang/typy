//! The `int` arithmetic generated code calls, with Python's results (EXP-02, EXP-03). Inline words that do not overflow take a fast path.

use super::value::{Integer, View};
use crate::Raising;
use crate::exception::{ExceptionKind, raise};
use crate::word::{inline, inline_value, is_inline};

/// Returns the result of `fast` on two inline words when it fits inline, or `slow` on the full values.
///
/// # Safety
///
/// Both words are `int` words whose large integers are live.
unsafe fn binary(
    left: i64,
    right: i64,
    fast: impl Fn(i64, i64) -> Option<i64>,
    slow: impl Fn(View<'_>, View<'_>) -> Integer,
) -> i64 {
    if is_inline(left)
        && is_inline(right)
        && let Some(word) = fast(inline_value(left), inline_value(right)).and_then(inline)
    {
        return word;
    }
    let (mut left_held, mut right_held) = ([0], [0]);
    // SAFETY: the caller passes `int` words whose large integers are live; the views read
    // their limbs in place.
    let (left, right) = unsafe {
        (
            View::of(left, &mut left_held),
            View::of(right, &mut right_held),
        )
    };
    slow(left, right).into_word()
}

/// Returns `left + right`.
///
/// # Safety
///
/// Both arguments are `int` words whose large integers are live.
#[allow(
    clippy::redundant_closure_for_method_calls,
    reason = "the method's path fixes its view's lifetime, and `binary` takes views of any"
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_int_add(left: i64, right: i64) -> i64 {
    // SAFETY: the caller's guarantee is passed on.
    unsafe { binary(left, right, i64::checked_add, |a, b| a.add(b)) }
}

/// Returns `left - right`.
///
/// # Safety
///
/// Both arguments are `int` words whose large integers are live.
#[allow(
    clippy::redundant_closure_for_method_calls,
    reason = "the method's path fixes its view's lifetime, and `binary` takes views of any"
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_int_subtract(left: i64, right: i64) -> i64 {
    // SAFETY: the caller's guarantee is passed on.
    unsafe { binary(left, right, i64::checked_sub, |a, b| a.subtract(b)) }
}

/// Returns `left * right`.
///
/// # Safety
///
/// Both arguments are `int` words whose large integers are live.
#[allow(
    clippy::redundant_closure_for_method_calls,
    reason = "the method's path fixes its view's lifetime, and `binary` takes views of any"
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_int_multiply(left: i64, right: i64) -> i64 {
    // SAFETY: the caller's guarantee is passed on.
    unsafe { binary(left, right, i64::checked_mul, |a, b| a.multiply(b)) }
}

/// Returns `-value`.
///
/// # Safety
///
/// `value` is an `int` word whose large integer is live.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_int_negate(value: i64) -> i64 {
    // SAFETY: the caller's guarantee is passed on.
    unsafe {
        binary(
            value,
            value,
            |a, _| a.checked_neg(),
            |a, _| Integer::from_i128(0).view().subtract(a),
        )
    }
}

/// Returns `abs(value)`.
///
/// # Safety
///
/// `value` is an `int` word whose large integer is live.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_int_absolute(value: i64) -> i64 {
    // SAFETY: the caller's guarantee is passed on.
    unsafe {
        binary(
            value,
            value,
            |a, _| a.checked_abs(),
            |a, _| {
                let positive = View {
                    negative: false,
                    magnitude: a.magnitude,
                };
                positive.add(Integer::from_i128(0).view())
            },
        )
    }
}

/// Compares two `int` words; `comparison` is a tagged `int` numbered as TyIR's `Comparison::code`.
///
/// # Safety
///
/// Both arguments are `int` words whose large integers are live.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_int_compare(left: i64, right: i64, comparison: i64) -> bool {
    let ordering = if is_inline(left) && is_inline(right) {
        inline_value(left).cmp(&inline_value(right))
    } else {
        let (mut left_held, mut right_held) = ([0], [0]);
        // SAFETY: the caller passes `int` words whose large integers are live; the views read
        // their limbs in place.
        unsafe { View::of(left, &mut left_held).compare(View::of(right, &mut right_held)) }
    };
    crate::compare::holds(ordering, comparison)
}

/// CPython 3.14 and later word every division by zero alike; the runtime follows the newest oracle (OI-14).
pub(crate) const ZERO_DIVISION: &str = "division by zero";

fn zero_division(message: &str) -> Raising<i64> {
    Raising::error(1, raise(ExceptionKind::ZeroDivisionError, message))
}

/// Returns `left // right`, rounded toward negative infinity.
///
/// # Safety
///
/// Both arguments are `int` words whose large integers are live.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_int_floor_divide(left: i64, right: i64) -> Raising<i64> {
    if Some(right) == inline(0) {
        return zero_division(ZERO_DIVISION);
    }
    // SAFETY: the caller's guarantee is passed on.
    Raising::ok(unsafe {
        binary(
            left,
            right,
            |a, b| {
                a.checked_div_euclid(b)
                    .map(|q| q - i64::from(b < 0 && a.rem_euclid(b) != 0))
            },
            |a, b| a.divide_floor(b).0,
        )
    })
}

/// Returns `left % right`, with the sign of `right`.
///
/// # Safety
///
/// Both arguments are `int` words whose large integers are live.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_int_modulo(left: i64, right: i64) -> Raising<i64> {
    if Some(right) == inline(0) {
        return zero_division(ZERO_DIVISION);
    }
    let fast = |a: i64, b: i64| {
        a.checked_rem_euclid(b)
            .map(|r| if b < 0 && r != 0 { r + b } else { r })
    };
    // SAFETY: the caller's guarantee is passed on.
    Raising::ok(unsafe { binary(left, right, fast, |a, b| a.divide_floor(b).1) })
}

/// Returns `base ** exponent` for a non-negative exponent, by repeated squaring (EXP-04 keeps the exponent non-negative).
///
/// # Safety
///
/// Both arguments are `int` words whose large integers are live.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_int_power(base: i64, exponent: i64) -> Raising<i64> {
    // SAFETY: the caller passes `int` words whose large integers are live.
    let (base, exponent) = unsafe { (Integer::from_word(base), Integer::from_word(exponent)) };
    if exponent.negative {
        return Raising::error(1, raise(ExceptionKind::ValueError, "negative exponent"));
    }
    let mut result = Integer::from_i128(1);
    let mut square = base;
    let limbs = exponent.magnitude.as_slice();
    let bits = super::natural::bit_length(limbs);
    for bit in 0..bits {
        let word = limbs[usize::try_from(bit / 64).unwrap_or(0)];
        if word >> (bit % 64) & 1 == 1 {
            result = result.multiply(&square);
        }
        if bit + 1 < bits {
            square = square.multiply(&square);
        }
    }
    Raising::ok(result.into_word())
}

/// Returns `pow(base, exponent, modulus)` as CPython computes it: the power reduced after
/// every step, a result with the sign of the modulus, and for a negative exponent the power of
/// the base's inverse. A modulus of zero, or a base with no inverse for a negative exponent,
/// raises `ValueError` (LIB-28).
///
/// # Safety
///
/// The arguments are `int` words whose large integers are live.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_int_power_modulo(
    base: i64,
    exponent: i64,
    modulus: i64,
) -> Raising<i64> {
    // SAFETY: the caller passes `int` words whose large integers are live.
    let (base, exponent, modulus) = unsafe {
        (
            Integer::from_word(base),
            Integer::from_word(exponent),
            Integer::from_word(modulus),
        )
    };
    if modulus.is_zero() {
        return value_error("pow() 3rd argument cannot be 0");
    }
    let negative_result = modulus.negative;
    let modulus = if negative_result {
        modulus.negate()
    } else {
        modulus
    };
    let one = Integer::from_i128(1);
    // Every number is 0 modulo 1, also the empty product of `pow(b, 0, 1)`.
    if modulus.compare(&one) == core::cmp::Ordering::Equal {
        return Raising::ok(Integer::from_i128(0).into_word());
    }
    let (_, mut square) = base.divide_floor(&modulus);
    let exponent = if exponent.negative {
        let Some(inverse) = inverse(&square, &modulus) else {
            return value_error("base is not invertible for the given modulus");
        };
        square = inverse;
        exponent.negate()
    } else {
        exponent
    };
    let mut result = one;
    let limbs = exponent.magnitude.as_slice();
    let bits = super::natural::bit_length(limbs);
    for bit in 0..bits {
        let word = limbs[usize::try_from(bit / 64).unwrap_or(0)];
        if word >> (bit % 64) & 1 == 1 {
            result = result.multiply(&square).divide_floor(&modulus).1;
        }
        if bit + 1 < bits {
            square = square.multiply(&square).divide_floor(&modulus).1;
        }
    }
    if negative_result && !result.is_zero() {
        result = result.subtract(&modulus);
    }
    Raising::ok(result.into_word())
}

/// Returns the inverse of `value` modulo `modulus`, in `[0, modulus)`, by the extended
/// Euclidean algorithm, or `None` when the two have a common divisor other than 1; `value` is
/// in `[0, modulus)` and `modulus` is above 1.
fn inverse(value: &Integer, modulus: &Integer) -> Option<Integer> {
    let zero = Integer::from_i128(0);
    let (mut previous, mut remainder) = (modulus.add(&zero), value.add(&zero));
    let (mut previous_factor, mut factor) = (Integer::from_i128(0), Integer::from_i128(1));
    while !remainder.is_zero() {
        let (quotient, next) = previous.divide_floor(&remainder);
        previous = core::mem::replace(&mut remainder, next);
        let next_factor = previous_factor.subtract(&quotient.multiply(&factor));
        previous_factor = core::mem::replace(&mut factor, next_factor);
    }
    if previous.compare(&Integer::from_i128(1)) != core::cmp::Ordering::Equal {
        return None;
    }
    Some(previous_factor.divide_floor(modulus).1)
}

fn value_error(message: &str) -> Raising<i64> {
    Raising::error(1, raise(ExceptionKind::ValueError, message))
}

#[cfg(test)]
mod tests;
