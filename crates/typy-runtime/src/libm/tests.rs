//! Tests of the C maths library functions and sign operations on doubles.

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
