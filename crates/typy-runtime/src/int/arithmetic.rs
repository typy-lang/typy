//! The `int` arithmetic generated code calls, with Python's results (EXP-02, EXP-03). Inline words that do not overflow take a fast path.

use super::value::Integer;
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
    slow: impl Fn(&Integer, &Integer) -> Integer,
) -> i64 {
    if is_inline(left)
        && is_inline(right)
        && let Some(word) = fast(inline_value(left), inline_value(right)).and_then(inline)
    {
        return word;
    }
    // SAFETY: the caller passes `int` words whose large integers are live.
    let (left, right) = unsafe { (Integer::from_word(left), Integer::from_word(right)) };
    slow(&left, &right).into_word()
}

/// Returns `left + right`.
///
/// # Safety
///
/// Both arguments are `int` words whose large integers are live.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_int_add(left: i64, right: i64) -> i64 {
    // SAFETY: the caller's guarantee is passed on.
    unsafe { binary(left, right, i64::checked_add, Integer::add) }
}

/// Returns `left - right`.
///
/// # Safety
///
/// Both arguments are `int` words whose large integers are live.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_int_subtract(left: i64, right: i64) -> i64 {
    // SAFETY: the caller's guarantee is passed on.
    unsafe { binary(left, right, i64::checked_sub, Integer::subtract) }
}

/// Returns `left * right`.
///
/// # Safety
///
/// Both arguments are `int` words whose large integers are live.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_int_multiply(left: i64, right: i64) -> i64 {
    // SAFETY: the caller's guarantee is passed on.
    unsafe { binary(left, right, i64::checked_mul, Integer::multiply) }
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
            |a, _| Integer::from_i128(0).subtract(a),
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
                if a.negative {
                    Integer::from_i128(0).subtract(a)
                } else {
                    a.add(&Integer::from_i128(0))
                }
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
        // SAFETY: the caller passes `int` words whose large integers are live.
        unsafe { Integer::from_word(left).compare(&Integer::from_word(right)) }
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

#[cfg(test)]
mod tests;
