//! `round`, with CPython's results: half to even on the exact value of a `float` or an `int`,
//! and a result read back from the rounded decimal value with one correct rounding (LIB-02).

use super::convert::truncate;
use super::ratio::ratio;
use crate::Raising;
use crate::exception::{ExceptionKind, raise};
use crate::int::limbs::Limbs;
use crate::int::natural::{bit_length, divide, multiply, shift_left};
use crate::int::value::Integer;
use crate::libm::{floor, fmod};
use crate::word::{inline_value, is_inline};

/// The largest number of digits after the point that can change a double; beyond it a double
/// rounds to itself. It is `(53 - (-1021)) * 0.30103`, as CPython's `NDIGITS_MAX`.
const MOST_DIGITS: i64 = 323;
/// The smallest number of digits below which a double rounds to zero, CPython's `NDIGITS_MIN`.
const FEWEST_DIGITS: i64 = -308;

/// Returns `round(value)`: the nearest integer, a half to the even one, as a new `int` word.
#[allow(
    clippy::float_cmp,
    reason = "a fraction of exactly one half is a tie, which rounds to the even integer"
)]
#[unsafe(no_mangle)]
pub extern "C" fn typy_rt_float_round(value: f64) -> Raising<i64> {
    if value.is_nan() {
        let error = raise(
            ExceptionKind::ValueError,
            "cannot convert float NaN to integer",
        );
        return Raising::error(1, error);
    }
    if value.is_infinite() {
        let error = raise(
            ExceptionKind::OverflowError,
            "cannot convert float infinity to integer",
        );
        return Raising::error(1, error);
    }
    let below = floor(value);
    // `value - below` is exact: both are doubles of one binade or `below` is integral and the
    // value is not.
    let fraction = value - below;
    let rounded = if fraction > 0.5 || (fraction == 0.5 && fmod(below, 2.0) != 0.0) {
        below + 1.0
    } else {
        below
    };
    Raising::ok(truncate(rounded).into_word())
}

/// Returns `10 ** exponent`.
pub(crate) fn power_of_ten(exponent: u64) -> Limbs {
    let mut result = Limbs::from_slice(&[1]);
    let mut base = Limbs::from_slice(&[10]);
    let mut remaining = exponent;
    while remaining > 0 {
        if remaining & 1 == 1 {
            result = multiply(result.as_slice(), base.as_slice());
        }
        remaining >>= 1;
        if remaining > 0 {
            base = multiply(base.as_slice(), base.as_slice());
        }
    }
    result
}

/// Returns `numerator / denominator` rounded to an integer, a half to the even one.
pub(crate) fn round_quotient(numerator: &[u64], denominator: &[u64]) -> Limbs {
    let (quotient, remainder) = divide(numerator, denominator);
    let twice = shift_left(remainder.as_slice(), 1);
    let order = crate::int::natural::compare(twice.as_slice(), denominator);
    let odd = quotient.as_slice().first().is_some_and(|low| low & 1 == 1);
    if order.is_gt() || (order.is_eq() && odd) {
        crate::int::natural::add(quotient.as_slice(), &[1])
    } else {
        quotient
    }
}

/// Returns `round(value, digits)` for a `float`: the value rounded half to even at `digits`
/// places after the point, or before it when `digits` is negative, as CPython rounds the exact
/// value with `_Py_dg_dtoa` and reads the result back. A result too large for a double raises
/// `OverflowError`.
///
/// # Safety
///
/// `digits` is an `int` word whose large integer is live.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_float_round_digits(value: f64, digits: i64) -> Raising<f64> {
    if !value.is_finite() {
        return Raising::ok(value);
    }
    let digits = if is_inline(digits) {
        inline_value(digits)
    } else if
    // SAFETY: the caller passes a live large integer.
    unsafe { crate::int::is_negative(digits) } {
        i64::MIN
    } else {
        i64::MAX
    };
    if digits > MOST_DIGITS {
        return Raising::ok(value);
    }
    if digits < FEWEST_DIGITS {
        return Raising::ok(0.0 * value);
    }
    let bits = value.to_bits();
    let biased = i64::try_from((bits >> 52) & 0x7ff).unwrap_or(0);
    let fraction = bits & ((1 << 52) - 1);
    let (mantissa, exponent) = if biased == 0 {
        (fraction, -1074)
    } else {
        (fraction | (1 << 52), biased - 1075)
    };
    let scale = power_of_ten(digits.unsigned_abs());
    let lift = shift_left(&[mantissa], exponent.max(0).unsigned_abs());
    let drop = shift_left(&[1], (-exponent).max(0).unsigned_abs());
    let magnitude = if digits >= 0 {
        let numerator = multiply(lift.as_slice(), scale.as_slice());
        let rounded = round_quotient(numerator.as_slice(), drop.as_slice());
        ratio(rounded.as_slice(), scale.as_slice())
    } else {
        let denominator = multiply(drop.as_slice(), scale.as_slice());
        let rounded = round_quotient(lift.as_slice(), denominator.as_slice());
        ratio(
            multiply(rounded.as_slice(), scale.as_slice()).as_slice(),
            &[1],
        )
    };
    match magnitude {
        Some(magnitude) => Raising::ok(magnitude.copysign(value)),
        None => Raising::error(
            0.0,
            raise(
                ExceptionKind::OverflowError,
                "rounded value too large to represent",
            ),
        ),
    }
}

/// Returns `round(value, digits)` for an `int`, as a new `int` word: the value itself for
/// `digits` of zero or more, and otherwise the nearest multiple of `10 ** -digits`, a half to
/// the even one, as CPython's `long_round` gives it.
///
/// # Safety
///
/// Both are `int` words whose large integers are live.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_int_round_digits(value: i64, digits: i64) -> i64 {
    // SAFETY: the caller passes live words.
    let (integer, negative_digits) = unsafe {
        (
            Integer::from_word(value),
            if is_inline(digits) {
                inline_value(digits) < 0
            } else {
                crate::int::is_negative(digits)
            },
        )
    };
    if !negative_digits {
        // SAFETY: the caller passes a live word.
        unsafe { crate::refcount::typy_rt_int_increment(value) };
        return value;
    }
    let places = if is_inline(digits) {
        inline_value(digits).unsigned_abs()
    } else {
        u64::MAX
    };
    // A value with fewer decimal digits than `places` is below half of `10 ** places`, so it
    // rounds to zero; 0.302 is above log10(2), so this never misjudges a value.
    let decimal_digits = bit_length(integer.magnitude.as_slice()) * 302 / 1000 + 1;
    if places > decimal_digits + 1 {
        return Integer::from_i128(0).into_word();
    }
    let scale = power_of_ten(places);
    let rounded = round_quotient(integer.magnitude.as_slice(), scale.as_slice());
    let mut magnitude = multiply(rounded.as_slice(), scale.as_slice());
    magnitude.trim();
    Integer {
        negative: integer.negative && !magnitude.is_zero(),
        magnitude,
    }
    .into_word()
}

#[cfg(test)]
mod tests;
