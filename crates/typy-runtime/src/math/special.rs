//! The functions of `math` that CPython computes itself rather than leaving to the C
//! library: `pow`, `fmod`, `remainder`, `ldexp`, `ulp`, `nextafter`, `fma`, `isclose` and
//! `hypot`, ported from CPython 3.15's `mathmodule.c`.
#![allow(
    clippy::float_cmp,
    reason = "CPython's algorithms compare doubles exactly, as IEEE 754 defines"
)]

use super::c::{self, call2};
use super::real::math_2;
use super::{domain_error, ok, range_error};
use crate::Raising;
use crate::exception::{ExceptionKind, raise};
use crate::libm::{absolute, copy_sign};

/// `math.pow(x, y)`: IEEE special values directly, the C library for finite values, and a
/// domain error for a NaN or for zero to a negative power.
#[unsafe(no_mangle)]
pub extern "C" fn typy_rt_math_pow(x: f64, y: f64) -> Raising<f64> {
    if !x.is_finite() || !y.is_finite() {
        return ok(if x.is_nan() {
            if y == 0.0 { 1.0 } else { x }
        } else if y.is_nan() {
            if x == 1.0 { 1.0 } else { y }
        } else if x.is_infinite() {
            let odd = y.is_finite() && crate::libm::fmod(absolute(y), 2.0) == 1.0;
            if y > 0.0 {
                if odd { x } else { absolute(x) }
            } else if y == 0.0 {
                1.0
            } else if odd {
                copy_sign(0.0, x)
            } else {
                0.0
            }
        } else if absolute(x) == 1.0 {
            1.0
        } else if (y > 0.0 && absolute(x) > 1.0) || (y < 0.0 && absolute(x) < 1.0) {
            absolute(y)
        } else {
            0.0
        });
    }
    let result = crate::libm::pow(x, y);
    if result.is_nan() || (result.is_infinite() && x == 0.0) {
        return domain_error();
    }
    if result.is_infinite() {
        return range_error();
    }
    ok(result)
}

/// `math.fmod(x, y)`: `x` itself for an infinite `y`, else the C library's `fmod`.
#[unsafe(no_mangle)]
pub extern "C" fn typy_rt_math_fmod(x: f64, y: f64) -> Raising<f64> {
    if y.is_infinite() && x.is_finite() {
        return ok(x);
    }
    let result = crate::libm::fmod(x, y);
    if result.is_nan() && !x.is_nan() && !y.is_nan() {
        return domain_error();
    }
    ok(result)
}

/// `math.remainder(x, y)`: `x` minus the nearest multiple of `y`, an even one on a tie,
/// computed exactly.
#[unsafe(no_mangle)]
pub extern "C" fn typy_rt_math_remainder(x: f64, y: f64) -> Raising<f64> {
    math_2(x, y, remainder(x, y))
}

fn remainder(x: f64, y: f64) -> f64 {
    if x.is_finite() && y.is_finite() {
        if y == 0.0 {
            return f64::NAN;
        }
        let (absolute_x, absolute_y) = (absolute(x), absolute(y));
        let modulus = crate::libm::fmod(absolute_x, absolute_y);
        // Comparing with the complement avoids halving `absolute_y`, which may lose bits.
        let complement = absolute_y - modulus;
        let result = if modulus < complement {
            modulus
        } else if modulus > complement {
            -complement
        } else {
            modulus - 2.0 * crate::libm::fmod(0.5 * (absolute_x - modulus), absolute_y)
        };
        return copy_sign(1.0, x) * result;
    }
    if x.is_nan() {
        return x;
    }
    if y.is_nan() {
        return y;
    }
    if x.is_infinite() {
        return f64::NAN;
    }
    x
}

/// `math.ldexp(x, i)`, with `i` given as an `int` word: an exponent beyond an `int` of C
/// overflows or underflows as CPython clamps it.
///
/// # Safety
///
/// `exponent` is an `int` word whose large integer is live.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_math_ldexp(x: f64, exponent: i64) -> Raising<f64> {
    // SAFETY: the caller's guarantee.
    let integer = unsafe { super::integer(exponent) };
    let clamped = if integer.negative {
        i128::MIN
    } else {
        i128::MAX
    };
    let exponent = integer.to_i128().unwrap_or(clamped);
    if x == 0.0 || !x.is_finite() {
        return ok(x);
    }
    if exponent > i128::from(i32::MAX) {
        return range_error();
    }
    if exponent < i128::from(i32::MIN) {
        return ok(copy_sign(0.0, x));
    }
    let result = crate::libm::ldexp(x, i64::try_from(exponent).unwrap_or(0));
    if result.is_infinite() {
        return range_error();
    }
    ok(result)
}

/// `math.ulp(x)`: the value of the least significant bit of `x`.
#[unsafe(no_mangle)]
pub extern "C" fn typy_rt_math_ulp(x: f64) -> f64 {
    if x.is_nan() {
        return x;
    }
    let x = absolute(x);
    if x.is_infinite() {
        return x;
    }
    let next = call2(c::nextafter, x, f64::INFINITY);
    if next.is_infinite() {
        return x - call2(c::nextafter, x, f64::NEG_INFINITY);
    }
    next - x
}

/// `math.nextafter(x, y, steps=steps)`, with `steps` an `int` word; a negative number of
/// steps is a `ValueError`, and more than fit in 64 bits count as the most.
///
/// # Safety
///
/// `steps` is an `int` word whose large integer is live.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_math_nextafter(x: f64, y: f64, steps: i64) -> Raising<f64> {
    // SAFETY: the caller's guarantee.
    let integer = unsafe { super::integer(steps) };
    if integer.negative && !integer.is_zero() {
        return Raising::error(
            0.0,
            raise(
                ExceptionKind::ValueError,
                "steps must be a non-negative integer",
            ),
        );
    }
    let steps = integer
        .to_i128()
        .and_then(|value| u64::try_from(value).ok())
        .unwrap_or(u64::MAX);
    ok(step(x, y, steps))
}

/// The sign bit of a double.
const SIGN: u64 = 1 << 63;

/// Returns the float `steps` steps from `x` towards `y`, as CPython's `nextafter` counts them
/// on the bits of the two.
fn step(x: f64, y: f64, steps: u64) -> f64 {
    if steps == 0 || x.is_nan() {
        return x;
    }
    if y.is_nan() {
        return y;
    }
    let (bits_x, bits_y) = (x.to_bits(), y.to_bits());
    if bits_x == bits_y {
        return x;
    }
    let (magnitude_x, magnitude_y) = (bits_x & !SIGN, bits_y & !SIGN);
    if (bits_x ^ bits_y) & SIGN != 0 {
        if magnitude_x + magnitude_y <= steps {
            return y;
        }
        if magnitude_x < steps {
            return f64::from_bits((bits_y & SIGN) | (steps - magnitude_x));
        }
        return f64::from_bits(bits_x - steps);
    }
    if magnitude_x > magnitude_y {
        if magnitude_x - magnitude_y >= steps {
            return f64::from_bits(bits_x - steps);
        }
        return y;
    }
    if magnitude_y - magnitude_x >= steps {
        return f64::from_bits(bits_x + steps);
    }
    y
}

/// `math.fma(x, y, z)`: `x * y + z` rounded once.
#[unsafe(no_mangle)]
pub extern "C" fn typy_rt_math_fma(x: f64, y: f64, z: f64) -> Raising<f64> {
    // SAFETY: `fma` has no preconditions.
    let result = unsafe { c::fma(x, y, z) };
    if result.is_finite() {
        return ok(result);
    }
    if result.is_nan() {
        if !x.is_nan() && !y.is_nan() && !z.is_nan() {
            return Raising::error(
                0.0,
                raise(ExceptionKind::ValueError, "invalid operation in fma"),
            );
        }
    } else if x.is_finite() && y.is_finite() && z.is_finite() {
        return Raising::error(0.0, raise(ExceptionKind::OverflowError, "overflow in fma"));
    }
    ok(result)
}

/// `math.isclose(a, b, rel_tol=rel_tol, abs_tol=abs_tol)`.
#[unsafe(no_mangle)]
pub extern "C" fn typy_rt_math_isclose(
    a: f64,
    b: f64,
    relative: f64,
    absolute_tolerance: f64,
) -> Raising<bool> {
    if relative < 0.0 || absolute_tolerance < 0.0 {
        return Raising::error(
            false,
            raise(ExceptionKind::ValueError, "tolerances must be non-negative"),
        );
    }
    if a == b {
        return Raising::ok(true);
    }
    if a.is_infinite() || b.is_infinite() {
        return Raising::ok(false);
    }
    let difference = absolute(b - a);
    Raising::ok(
        difference <= absolute(relative * b)
            || difference <= absolute(relative * a)
            || difference <= absolute_tolerance,
    )
}

/// `math.hypot(*coordinates)` of a new list of `float`s, by CPython's `vector_norm`:
/// lossless scaling, exact squares, compensated summation and a corrected square root.
///
/// # Safety
///
/// `coordinates` points to a live list of `float`s.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_math_hypot(coordinates: *const crate::list::ListObject) -> f64 {
    // SAFETY: the caller's guarantee.
    let list = unsafe { &*coordinates };
    let count = list.length;
    let size = crate::allocator::array_size(count.max(1), size_of::<f64>(), 0);
    #[allow(
        clippy::cast_ptr_alignment,
        reason = "malloc returns memory aligned for every type"
    )]
    let buffer = crate::allocator::allocate(size).cast::<f64>();
    // SAFETY: the buffer holds `count` doubles, and each index is inside the list, whose
    // items are the bits of `float`s.
    let vector = unsafe {
        for index in 0..count {
            buffer.add(index).write(f64::from_bits(list.item(index)));
        }
        core::slice::from_raw_parts_mut(buffer, count)
    };
    let mut max = 0.0_f64;
    let mut found_nan = false;
    for x in vector.iter_mut() {
        *x = absolute(*x);
        found_nan |= x.is_nan();
        if *x > max {
            max = *x;
        }
    }
    let result = vector_norm(vector, max, found_nan);
    crate::allocator::free(buffer.cast());
    result
}

/// A product or a sum held exactly as a high and a low part.
#[derive(Clone, Copy)]
struct DoubleLength {
    high: f64,
    low: f64,
}

/// Returns `a + b` exactly, when `|a| >= |b|`.
fn fast_sum(a: f64, b: f64) -> DoubleLength {
    let high = a + b;
    DoubleLength {
        high,
        low: (a - high) + b,
    }
}

/// Returns `x * y` exactly, with `fma` giving the low part.
fn product(x: f64, y: f64) -> DoubleLength {
    let high = x * y;
    // SAFETY: `fma` has no preconditions.
    let low = unsafe { c::fma(x, y, -high) };
    DoubleLength { high, low }
}

pub(super) fn vector_norm(vector: &mut [f64], max: f64, found_nan: bool) -> f64 {
    if max.is_infinite() {
        return max;
    }
    if found_nan {
        return f64::NAN;
    }
    if max == 0.0 || vector.len() <= 1 {
        return max;
    }
    let (_, max_exponent) = c::split(max);
    if max_exponent < -1023 {
        for x in vector.iter_mut() {
            *x /= f64::MIN_POSITIVE;
        }
        return f64::MIN_POSITIVE * vector_norm(vector, max / f64::MIN_POSITIVE, found_nan);
    }
    let scale = crate::libm::ldexp(1.0, i64::from(-max_exponent));
    let (mut sum, mut fraction_1, mut fraction_2) = (1.0_f64, 0.0_f64, 0.0_f64);
    for x in vector.iter() {
        let x = *x * scale;
        let square = product(x, x);
        let added = fast_sum(sum, square.high);
        sum = added.high;
        fraction_1 += square.low;
        fraction_2 += added.low;
    }
    // SAFETY: `sqrt` has no preconditions.
    let mut root = unsafe { c::sqrt(sum - 1.0 + (fraction_1 + fraction_2)) };
    let square = product(-root, root);
    let added = fast_sum(sum, square.high);
    sum = added.high;
    fraction_1 += square.low;
    fraction_2 += added.low;
    let x = sum - 1.0 + (fraction_1 + fraction_2);
    root += x / (2.0 * root);
    root / scale
}
