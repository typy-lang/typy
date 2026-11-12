//! The functions of the C maths library that `math` calls.
//!
//! Rust's `compiler_builtins` defines weak copies of some of them, such as `cbrt`, whose
//! results differ from the C library's in the last bit. Naming the C library here puts it
//! before `compiler_builtins` when a Rust binary of the runtime is linked, and the program's
//! link names it before the runtime archive, so CPython's C library is the one called.

#[link(name = "m")]
unsafe extern "C" {
    pub(super) fn acos(x: f64) -> f64;
    pub(super) fn acosh(x: f64) -> f64;
    pub(super) fn asin(x: f64) -> f64;
    pub(super) fn asinh(x: f64) -> f64;
    pub(super) fn atan(x: f64) -> f64;
    pub(super) fn atan2(y: f64, x: f64) -> f64;
    pub(super) fn atanh(x: f64) -> f64;
    pub(super) fn cbrt(x: f64) -> f64;
    pub(super) fn ceil(x: f64) -> f64;
    pub(super) fn copysign(x: f64, y: f64) -> f64;
    pub(super) fn cos(x: f64) -> f64;
    pub(super) fn cosh(x: f64) -> f64;
    pub(super) fn erf(x: f64) -> f64;
    pub(super) fn erfc(x: f64) -> f64;
    pub(super) fn exp(x: f64) -> f64;
    pub(super) fn exp2(x: f64) -> f64;
    pub(super) fn expm1(x: f64) -> f64;
    pub(super) fn fma(x: f64, y: f64, z: f64) -> f64;
    pub(super) fn frexp(x: f64, exponent: *mut i32) -> f64;
    pub(super) fn log(x: f64) -> f64;
    pub(super) fn log10(x: f64) -> f64;
    pub(super) fn log1p(x: f64) -> f64;
    pub(super) fn log2(x: f64) -> f64;
    pub(super) fn modf(x: f64, integral: *mut f64) -> f64;
    pub(super) fn nextafter(x: f64, y: f64) -> f64;
    pub(super) fn sin(x: f64) -> f64;
    pub(super) fn sinh(x: f64) -> f64;
    pub(super) fn sqrt(x: f64) -> f64;
    pub(super) fn tan(x: f64) -> f64;
    pub(super) fn tanh(x: f64) -> f64;
}

/// Calls a C function of one `double`, which has no preconditions.
pub(super) fn call(function: unsafe extern "C" fn(f64) -> f64, x: f64) -> f64 {
    // SAFETY: every function of the C maths library taking one double is defined for all of
    // them.
    unsafe { function(x) }
}

/// Calls a C function of two `double`s, which has no preconditions.
pub(super) fn call2(function: unsafe extern "C" fn(f64, f64) -> f64, x: f64, y: f64) -> f64 {
    // SAFETY: as `call`.
    unsafe { function(x, y) }
}

/// Returns C's `frexp`: the mantissa and the exponent.
pub(super) fn split(x: f64) -> (f64, i32) {
    let mut exponent = 0;
    // SAFETY: the pointer is to a live local.
    let mantissa = unsafe { frexp(x, &raw mut exponent) };
    (mantissa, exponent)
}

/// Returns C's `modf`: the fraction and the integral part.
pub(super) fn parts(x: f64) -> (f64, f64) {
    let mut integral = 0.0;
    // SAFETY: the pointer is to a live local.
    let fraction = unsafe { modf(x, &raw mut integral) };
    (fraction, integral)
}
