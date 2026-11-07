//! Converts between `int` and `float` exactly, and compares them exactly, as CPython does.

use core::cmp::Ordering;

use super::ratio::ratio;
use crate::Raising;
use crate::exception::{ExceptionKind, raise};
use crate::int::ZERO_DIVISION;
use crate::int::natural::shift_left;
use crate::int::value::Integer;
use crate::word::{inline, inline_value, is_inline};

/// Integers up to 2^53 in size convert to a double exactly.
const EXACT_LIMIT: i64 = 1 << 53;

/// Returns the inline value of `word` when it converts to a double exactly.
fn exact_small(word: i64) -> Option<i64> {
    let value = is_inline(word).then(|| inline_value(word))?;
    (value.unsigned_abs() <= EXACT_LIMIT.unsigned_abs()).then_some(value)
}

#[allow(
    clippy::cast_precision_loss,
    reason = "callers pass values of at most 53 bits, which convert exactly"
)]
const fn exact(value: i64) -> f64 {
    value as f64
}

fn signed(negative: bool, magnitude: f64) -> f64 {
    if negative { -magnitude } else { magnitude }
}

/// Returns `float(value)`, rounded to nearest with ties to even.
///
/// # Safety
///
/// `value` is an `int` word whose large integer is live.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_int_to_float(value: i64) -> Raising<f64> {
    if let Some(small) = exact_small(value) {
        return Raising::ok(exact(small));
    }
    // SAFETY: the caller passes an `int` word whose large integer is live.
    let integer = unsafe { Integer::from_word(value) };
    match ratio(integer.magnitude.as_slice(), &[1]) {
        Some(magnitude) => Raising::ok(signed(integer.negative, magnitude)),
        None => Raising::error(
            0.0,
            raise(
                ExceptionKind::OverflowError,
                "int too large to convert to float",
            ),
        ),
    }
}

/// Returns `left / right`, rounded once to the nearest double.
///
/// # Safety
///
/// Both arguments are `int` words whose large integers are live.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_int_true_divide(left: i64, right: i64) -> Raising<f64> {
    if Some(right) == inline(0) {
        return Raising::error(0.0, raise(ExceptionKind::ZeroDivisionError, ZERO_DIVISION));
    }
    if let (Some(numerator), Some(denominator)) = (exact_small(left), exact_small(right)) {
        return Raising::ok(exact(numerator) / exact(denominator));
    }
    // SAFETY: the caller passes `int` words whose large integers are live.
    let (numerator, denominator) = unsafe { (Integer::from_word(left), Integer::from_word(right)) };
    let negative = numerator.negative != denominator.negative;
    match ratio(
        numerator.magnitude.as_slice(),
        denominator.magnitude.as_slice(),
    ) {
        Some(magnitude) => Raising::ok(signed(negative, magnitude)),
        None => Raising::error(
            0.0,
            raise(
                ExceptionKind::OverflowError,
                "integer division result too large for a float",
            ),
        ),
    }
}

/// Returns the integral part of a finite double, exactly.
fn truncate(value: f64) -> Integer {
    let bits = value.to_bits();
    let biased = i64::try_from((bits >> 52) & 0x7ff).unwrap_or(0);
    let exponent = biased - 1075;
    if exponent < 0 {
        // Below 2^53 in size, the integral part fits a machine integer.
        #[allow(
            clippy::cast_possible_truncation,
            reason = "the value is below 2^53 in size, and `as` truncates toward zero"
        )]
        return Integer::from_i128(value as i128);
    }
    let mantissa = (bits & ((1 << 52) - 1)) | (1 << 52);
    Integer {
        negative: value < 0.0,
        magnitude: shift_left(&[mantissa], exponent.unsigned_abs()),
    }
}

/// Returns `int(value)`: the value rounded toward zero.
#[unsafe(no_mangle)]
pub extern "C" fn typy_rt_float_to_int(value: f64) -> Raising<i64> {
    if value.is_nan() {
        return Raising::error(
            1,
            raise(
                ExceptionKind::ValueError,
                "cannot convert float NaN to integer",
            ),
        );
    }
    if value.is_infinite() {
        return Raising::error(
            1,
            raise(
                ExceptionKind::OverflowError,
                "cannot convert float infinity to integer",
            ),
        );
    }
    Raising::ok(truncate(value).into_word())
}

/// Returns how an `int` orders against a finite double, exactly.
///
/// # Safety
///
/// `left` is an `int` word whose large integer is live.
unsafe fn order(left: i64, right: f64) -> Ordering {
    if let Some(small) = exact_small(left) {
        return exact(small).partial_cmp(&right).unwrap_or(Ordering::Equal);
    }
    // The integer is larger than 2^53 in size. A double below that orders like its integral
    // part, and a double above it is integral, so comparing integral parts is exact.
    // SAFETY: the caller passes an `int` word whose large integer is live.
    unsafe { Integer::from_word(left) }.compare(&truncate(right))
}

/// Returns whether `left` and `right` satisfy the comparison whose code is the tagged `int`
/// `comparison`, comparing the exact values; `nan` is unordered, so only `!=` holds.
///
/// # Safety
///
/// `left` is an `int` word whose large integer is live, and `comparison` is an `int` word.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_compare_int_float(left: i64, right: f64, comparison: i64) -> bool {
    if right.is_nan() {
        return Some(comparison) == inline(1);
    }
    let ordering = if right.is_infinite() {
        if right > 0.0 {
            Ordering::Less
        } else {
            Ordering::Greater
        }
    } else {
        // SAFETY: the caller's guarantee is passed on.
        unsafe { order(left, right) }
    };
    crate::compare::holds(ordering, comparison)
}

#[cfg(test)]
mod tests;
