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
mod tests {
    use super::{absolute, copy_sign, floor, fmod, is_sign_negative, ldexp, pow};

    #[test]
    #[cfg_attr(miri, ignore = "Miri cannot call the C maths library's `pow`")]
    fn the_c_functions_and_sign_operations_agree_with_rust() {
        assert_eq!(floor(-2.5).to_bits(), (-3.0_f64).to_bits());
        assert_eq!(fmod(-7.5, 2.0).to_bits(), (-1.5_f64).to_bits());
        assert_eq!(pow(2.0, 10.0).to_bits(), 1024.0_f64.to_bits());
        assert_eq!(ldexp(3.0, -1).to_bits(), 1.5_f64.to_bits());
        assert_eq!(ldexp(1.0, 1 << 40), f64::INFINITY);
        assert_eq!(absolute(-0.0).to_bits(), 0.0_f64.to_bits());
        assert_eq!(copy_sign(2.0, -0.0).to_bits(), (-2.0_f64).to_bits());
        assert!(is_sign_negative(-0.0));
        assert!(!is_sign_negative(0.0));
    }
}
