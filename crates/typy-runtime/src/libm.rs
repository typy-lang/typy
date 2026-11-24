//! The C maths library functions the runtime calls, and sign operations on the bits of a double.

mod c {
    unsafe extern "C" {
        pub(super) fn floor(value: f64) -> f64;
        pub(super) fn fmod(left: f64, right: f64) -> f64;
        pub(super) fn pow(base: f64, exponent: f64) -> f64;
        pub(super) fn ldexp(value: f64, exponent: i32) -> f64;
    }
}

const SIGN: u64 = 1 << 63;

/// Returns the largest integral value not above `value`.
pub(crate) fn floor(value: f64) -> f64 {
    // SAFETY: `floor` has no preconditions.
    unsafe { c::floor(value) }
}

/// Returns the remainder of `left / right` with the sign of `left`, as C's `fmod`.
pub(crate) fn fmod(left: f64, right: f64) -> f64 {
    // SAFETY: `fmod` has no preconditions.
    unsafe { c::fmod(left, right) }
}

/// Returns `base` raised to `exponent`, as C's `pow`.
pub(crate) fn pow(base: f64, exponent: f64) -> f64 {
    // SAFETY: `pow` has no preconditions.
    unsafe { c::pow(base, exponent) }
}

/// Returns `value * 2^exponent`, rounded once.
pub(crate) fn ldexp(value: f64, exponent: i64) -> f64 {
    let exponent = i32::try_from(exponent.clamp(-4000, 4000)).unwrap_or(0);
    // SAFETY: `ldexp` has no preconditions.
    unsafe { c::ldexp(value, exponent) }
}

/// Returns `value` without its sign.
pub(crate) const fn absolute(value: f64) -> f64 {
    f64::from_bits(value.to_bits() & !SIGN)
}

/// Returns `magnitude` with the sign of `sign`.
pub(crate) const fn copy_sign(magnitude: f64, sign: f64) -> f64 {
    f64::from_bits((magnitude.to_bits() & !SIGN) | (sign.to_bits() & SIGN))
}

/// Returns true when the sign bit is set, also for `-0.0`.
pub(crate) const fn is_sign_negative(value: f64) -> bool {
    value.to_bits() & SIGN != 0
}

#[cfg(test)]
mod tests;
