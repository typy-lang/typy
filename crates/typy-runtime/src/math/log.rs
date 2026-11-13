//! `math.log`, `math.log2` and `math.log10`, as CPython's `loghelper` computes them: the C
//! library for a `float`, and for an `int` too large for a `float`, the logarithm of its
//! mantissa plus its exponent times the logarithm of 2.
#![allow(
    clippy::float_cmp,
    reason = "CPython's algorithms compare doubles exactly, as IEEE 754 defines"
)]

use super::c::{self, call};
use super::ok;
use crate::Raising;
use crate::exception::{ExceptionKind, raise};
use crate::int::natural::{any_low_bits, bit_length, shift_right};

const POSITIVE: &str = "expected a positive input, got %s";

/// The C library's logarithm of a base, with CPython's handling of zero, negative numbers,
/// infinities and NaNs, which `math_1` then turns into errors.
fn logarithm(function: unsafe extern "C" fn(f64) -> f64, x: f64) -> f64 {
    if x.is_nan() || (x.is_infinite() && x > 0.0) {
        return x;
    }
    if x > 0.0 && x.is_finite() {
        return call(function, x);
    }
    if x == 0.0 {
        f64::NEG_INFINITY
    } else {
        f64::NAN
    }
}

/// Returns the logarithm of a `float` with `math_1`'s checks.
fn of_float(function: unsafe extern "C" fn(f64) -> f64, x: f64) -> Raising<f64> {
    let result = logarithm(function, x);
    if (result.is_nan() && !x.is_nan()) || (result.is_infinite() && x.is_finite()) {
        return super::domain_error_of(x, Some(POSITIVE));
    }
    ok(result)
}

/// Returns the logarithm of an `int` word, which must be positive.
///
/// # Safety
///
/// `value` is an `int` word whose large integer is live.
unsafe fn of_int(function: unsafe extern "C" fn(f64) -> f64, value: i64) -> Raising<f64> {
    // SAFETY: the caller's guarantee.
    let integer = unsafe { super::integer(value) };
    if integer.negative || integer.is_zero() {
        return Raising::error(
            0.0,
            raise(ExceptionKind::ValueError, "expected a positive input"),
        );
    }
    // SAFETY: the caller's guarantee.
    let converted = unsafe { crate::float::typy_rt_int_to_float(value) };
    if converted.exception.is_null() {
        return ok(call(function, converted.value));
    }
    // The int is too large for a float, so its logarithm comes from its mantissa.
    // SAFETY: the conversion's `OverflowError` is not raised, so it is released here.
    unsafe { crate::refcount::typy_rt_decrement(converted.exception.cast()) };
    let magnitude = integer.magnitude.as_slice();
    let bits = bit_length(magnitude);
    let (mantissa, exponent) = split(magnitude, bits);
    // SAFETY: `fma` has no preconditions.
    ok(unsafe { c::fma(call(function, 2.0), exponent, call(function, mantissa)) })
}

/// Returns `x` in `[0.5, 1)` and `e` with the integer about `x * 2**e`, as CPython's
/// `_PyLong_Frexp`: the top 55 bits, with a bit set when any lower bit is, round to the 53
/// of a `float` exactly once.
fn split(magnitude: &[u64], bits: u64) -> (f64, f64) {
    const KEPT: u64 = 55;
    let shift = bits - KEPT;
    let top = shift_right(magnitude, shift);
    let mut mantissa = top.as_slice().first().copied().unwrap_or(0);
    if any_low_bits(magnitude, shift) {
        mantissa |= 1;
    }
    #[allow(
        clippy::cast_precision_loss,
        reason = "the conversion rounds the 55 bits to a float once, as CPython does"
    )]
    let mut x = mantissa as f64 / (1_u64 << KEPT) as f64;
    #[allow(
        clippy::cast_precision_loss,
        reason = "a bit count fits a float exactly"
    )]
    let mut exponent = bits as f64;
    if x == 1.0 {
        x = 0.5;
        exponent += 1.0;
    }
    (x, exponent)
}

/// Defines the runtime functions of one logarithm, for a `float` and for an `int`.
macro_rules! logarithms {
    ($($float:ident, $int:ident, $function:path, $name:literal;)*) => {
        $(
            #[doc = concat!("`math.", $name, "(x)` of a `float`.")]
            #[unsafe(no_mangle)]
            pub extern "C" fn $float(x: f64) -> Raising<f64> {
                of_float($function, x)
            }

            #[doc = concat!("`math.", $name, "(x)` of an `int` word.")]
            ///
            /// # Safety
            ///
            /// `value` is an `int` word whose large integer is live.
            #[unsafe(no_mangle)]
            pub unsafe extern "C" fn $int(value: i64) -> Raising<f64> {
                // SAFETY: the caller's guarantee.
                unsafe { of_int($function, value) }
            }
        )*
    };
}

logarithms! {
    typy_rt_math_log_float, typy_rt_math_log_int, c::log, "log";
    typy_rt_math_log2_float, typy_rt_math_log2_int, c::log2, "log2";
    typy_rt_math_log10_float, typy_rt_math_log10_int, c::log10, "log10";
}

/// Returns `math.log(x, base)` from the two natural logarithms: their quotient.
#[unsafe(no_mangle)]
pub extern "C" fn typy_rt_math_log_divide(numerator: f64, denominator: f64) -> Raising<f64> {
    if denominator == 0.0 {
        return Raising::error(
            0.0,
            raise(ExceptionKind::ZeroDivisionError, crate::int::ZERO_DIVISION),
        );
    }
    ok(numerator / denominator)
}
