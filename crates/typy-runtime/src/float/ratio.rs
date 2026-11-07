//! Rounds the ratio of two natural numbers to the nearest double, as CPython's `long_true_divide`
//! does, so `int / int`, `float(int)` and `float(str)` round once and correctly.

#![allow(
    clippy::float_cmp,
    reason = "the overflow test asks whether rounding carried up to an exact power of two"
)]

use crate::int::natural::{any_low_bits, bit_length, divide, shift_left, shift_right};
use crate::libm::ldexp;

const MAX_EXPONENT: i64 = 1024;
const MIN_EXPONENT: i64 = -1021;
const MANTISSA_BITS: i64 = 53;

/// Returns `numerator / denominator` rounded to the nearest double, ties to even, or `None` when
/// the result is too large for a double. The denominator is not zero.
pub(crate) fn ratio(numerator: &[u64], denominator: &[u64]) -> Option<f64> {
    let numerator_bits = i64::try_from(bit_length(numerator)).unwrap_or(i64::MAX);
    if numerator_bits == 0 {
        return Some(0.0);
    }
    let difference = numerator_bits - i64::try_from(bit_length(denominator)).unwrap_or(i64::MAX);
    if difference > MAX_EXPONENT {
        return None;
    }
    if difference < MIN_EXPONENT - MANTISSA_BITS - 1 {
        return Some(0.0);
    }
    // The quotient is scaled to 55 or 56 bits, or fewer for a subnormal result, so two or three
    // bits below the mantissa decide the rounding together with the inexact flag.
    let shift = difference.max(MIN_EXPONENT) - MANTISSA_BITS - 2;
    let distance = shift.unsigned_abs();
    let (scaled, mut inexact) = if shift <= 0 {
        (shift_left(numerator, distance), false)
    } else {
        (
            shift_right(numerator, distance),
            any_low_bits(numerator, distance),
        )
    };
    let (quotient, remainder) = divide(scaled.as_slice(), denominator);
    inexact |= !remainder.is_zero();
    let quotient_bits = i64::try_from(bit_length(quotient.as_slice())).unwrap_or(0);
    let extra_bits = quotient_bits.max(MIN_EXPONENT - shift) - MANTISSA_BITS;
    let mask = 1_u64 << (extra_bits - 1);
    let mut low = quotient.as_slice().first().copied().unwrap_or(0) | u64::from(inexact);
    if low & mask != 0 && low & (3 * mask - 1) != 0 {
        low += mask;
    }
    low &= !(2 * mask - 1);
    #[allow(
        clippy::cast_precision_loss,
        reason = "the low bits are cleared, so the value has at most 53 significant bits"
    )]
    let value = low as f64;
    if shift + quotient_bits >= MAX_EXPONENT
        && (shift + quotient_bits > MAX_EXPONENT || value == ldexp(1.0, quotient_bits))
    {
        return None;
    }
    Some(ldexp(value, shift))
}

#[cfg(test)]
mod tests;
