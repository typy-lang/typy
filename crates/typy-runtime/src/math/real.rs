//! The functions of one or two `float`s that call the C maths library behind CPython's checks,
//! `math_1` and `math_2`.
#![allow(
    clippy::float_cmp,
    reason = "CPython's algorithms compare doubles exactly, as IEEE 754 defines"
)]

use super::c::{self, call, call2};
use super::{domain_error, domain_error_of, ok, range_error};
use crate::Raising;

/// Applies `function` to `x` with CPython's `math_1` checks: a NaN from a number is a domain
/// error, and an infinity from a finite number an overflow when `can_overflow`, otherwise a
/// domain error.
fn math_1(
    x: f64,
    function: unsafe extern "C" fn(f64) -> f64,
    can_overflow: bool,
    message: Option<&str>,
) -> Raising<f64> {
    let result = call(function, x);
    if result.is_nan() && !x.is_nan() {
        return domain_error_of(x, message);
    }
    if result.is_infinite() && x.is_finite() {
        return if can_overflow {
            range_error()
        } else {
            domain_error_of(x, message)
        };
    }
    ok(result)
}

/// Checks the result of a function of two `float`s as CPython's `math_2` does: a NaN from
/// numbers is a domain error. `math_2` also turns an infinity from finite numbers into an
/// overflow, which none of `atan2`, `copysign` and `remainder` can give.
pub(super) fn math_2(x: f64, y: f64, result: f64) -> Raising<f64> {
    if result.is_nan() && !x.is_nan() && !y.is_nan() {
        return domain_error();
    }
    ok(result)
}

const RANGE: &str = "expected a number in range from -1 up to 1, got %s";
const FINITE: &str = "expected a finite input, got %s";

/// Defines a runtime function of one `float` that applies a C function with `math_1`.
macro_rules! real {
    ($($(#[$doc:meta])* $name:ident: $function:path, $overflow:expr, $message:expr;)*) => {
        $(
            $(#[$doc])*
            #[unsafe(no_mangle)]
            pub extern "C" fn $name(x: f64) -> Raising<f64> {
                math_1(x, $function, $overflow, $message)
            }
        )*
    };
}

real! {
    /// `math.acos(x)`.
    typy_rt_math_acos: c::acos, false, Some(RANGE);
    /// `math.acosh(x)`.
    typy_rt_math_acosh: c::acosh, false, Some("expected argument value not less than 1, got %s");
    /// `math.asin(x)`.
    typy_rt_math_asin: c::asin, false, Some(RANGE);
    /// `math.asinh(x)`.
    typy_rt_math_asinh: c::asinh, false, None;
    /// `math.atan(x)`.
    typy_rt_math_atan: c::atan, false, None;
    /// `math.atanh(x)`.
    typy_rt_math_atanh: c::atanh, false, Some("expected a number between -1 and 1, got %s");
    /// `math.cbrt(x)`.
    typy_rt_math_cbrt: c::cbrt, false, None;
    /// `math.cos(x)`.
    typy_rt_math_cos: c::cos, false, Some(FINITE);
    /// `math.cosh(x)`.
    typy_rt_math_cosh: c::cosh, true, None;
    /// `math.erf(x)`, which never fails: the C library reports only an underflow, which
    /// CPython ignores.
    typy_rt_math_erf: c::erf, false, None;
    /// `math.erfc(x)`, which never fails, as `erf`.
    typy_rt_math_erfc: c::erfc, false, None;
    /// `math.exp(x)`.
    typy_rt_math_exp: c::exp, true, None;
    /// `math.exp2(x)`.
    typy_rt_math_exp2: c::exp2, true, None;
    /// `math.expm1(x)`.
    typy_rt_math_expm1: c::expm1, true, None;
    /// `math.log1p(x)`.
    typy_rt_math_log1p: c::log1p, false, Some("expected argument value > -1, got %s");
    /// `math.sin(x)`.
    typy_rt_math_sin: c::sin, false, Some(FINITE);
    /// `math.sinh(x)`.
    typy_rt_math_sinh: c::sinh, true, None;
    /// `math.sqrt(x)`.
    typy_rt_math_sqrt: c::sqrt, false, Some("expected a nonnegative input, got %s");
    /// `math.tan(x)`.
    typy_rt_math_tan: c::tan, false, Some(FINITE);
    /// `math.tanh(x)`.
    typy_rt_math_tanh: c::tanh, false, None;
}

/// `math.fabs(x)`, which never fails.
#[unsafe(no_mangle)]
pub extern "C" fn typy_rt_math_fabs(x: f64) -> f64 {
    crate::libm::absolute(x)
}

/// `math.degrees(x)`: `x` times `180 / pi`, without a check.
#[unsafe(no_mangle)]
pub extern "C" fn typy_rt_math_degrees(x: f64) -> f64 {
    x * (180.0 / core::f64::consts::PI)
}

/// `math.radians(x)`: `x` times `pi / 180`, without a check.
#[unsafe(no_mangle)]
pub extern "C" fn typy_rt_math_radians(x: f64) -> f64 {
    x * (core::f64::consts::PI / 180.0)
}

/// `math.atan2(y, x)`.
#[unsafe(no_mangle)]
pub extern "C" fn typy_rt_math_atan2(y: f64, x: f64) -> Raising<f64> {
    math_2(y, x, call2(c::atan2, y, x))
}

/// `math.copysign(x, y)`.
#[unsafe(no_mangle)]
pub extern "C" fn typy_rt_math_copysign(x: f64, y: f64) -> Raising<f64> {
    math_2(x, y, call2(c::copysign, x, y))
}

/// `math.fmax(x, y)`, which never fails: the other value for a NaN, and `x` when the two
/// are equal, as the CPython the oracle runs gives for `fmax(-0.0, 0.0)`, which C leaves open.
#[unsafe(no_mangle)]
pub extern "C" fn typy_rt_math_fmax(x: f64, y: f64) -> f64 {
    if x.is_nan() {
        return y;
    }
    if y.is_nan() || x >= y { x } else { y }
}

/// `math.fmin(x, y)`, which never fails, chosen as `fmax` chooses.
#[unsafe(no_mangle)]
pub extern "C" fn typy_rt_math_fmin(x: f64, y: f64) -> f64 {
    if x.is_nan() {
        return y;
    }
    if y.is_nan() || x <= y { x } else { y }
}

/// `math.floor(x)` of a `float` before it becomes an `int`.
#[unsafe(no_mangle)]
pub extern "C" fn typy_rt_math_floor(x: f64) -> f64 {
    crate::libm::floor(x)
}

/// `math.ceil(x)` of a `float` before it becomes an `int`.
#[unsafe(no_mangle)]
pub extern "C" fn typy_rt_math_ceil(x: f64) -> f64 {
    call(c::ceil, x)
}

/// The mantissa `math.frexp(x)` gives: `x` itself for zero, an infinity or a NaN.
#[unsafe(no_mangle)]
pub extern "C" fn typy_rt_math_frexp_mantissa(x: f64) -> f64 {
    if x.is_nan() || x.is_infinite() || x == 0.0 {
        return x;
    }
    c::split(x).0
}

/// The exponent `math.frexp(x)` gives, as an `int` word: 0 for zero, an infinity or a NaN.
#[unsafe(no_mangle)]
pub extern "C" fn typy_rt_math_frexp_exponent(x: f64) -> i64 {
    let exponent = if x.is_nan() || x.is_infinite() || x == 0.0 {
        0
    } else {
        c::split(x).1
    };
    crate::word::inline(i64::from(exponent)).unwrap_or(1)
}

/// The fraction `math.modf(x)` gives: a zero with the sign of an infinity, or a NaN itself.
#[unsafe(no_mangle)]
pub extern "C" fn typy_rt_math_modf_fraction(x: f64) -> f64 {
    if x.is_infinite() {
        return crate::libm::copy_sign(0.0, x);
    }
    if x.is_nan() {
        return x;
    }
    c::parts(x).0
}

/// The integral part `math.modf(x)` gives.
#[unsafe(no_mangle)]
pub extern "C" fn typy_rt_math_modf_integral(x: f64) -> f64 {
    if x.is_infinite() || x.is_nan() {
        return x;
    }
    c::parts(x).1
}

/// `math.isfinite(x)`.
#[unsafe(no_mangle)]
pub extern "C" fn typy_rt_math_isfinite(x: f64) -> bool {
    x.is_finite()
}

/// `math.isinf(x)`.
#[unsafe(no_mangle)]
pub extern "C" fn typy_rt_math_isinf(x: f64) -> bool {
    x.is_infinite()
}

/// `math.isnan(x)`.
#[unsafe(no_mangle)]
pub extern "C" fn typy_rt_math_isnan(x: f64) -> bool {
    x.is_nan()
}

/// `math.isnormal(x)`.
#[unsafe(no_mangle)]
pub extern "C" fn typy_rt_math_isnormal(x: f64) -> bool {
    x.is_normal()
}

/// `math.issubnormal(x)`.
#[unsafe(no_mangle)]
pub extern "C" fn typy_rt_math_issubnormal(x: f64) -> bool {
    x.is_subnormal()
}

/// `math.signbit(x)`.
#[unsafe(no_mangle)]
pub extern "C" fn typy_rt_math_signbit(x: f64) -> bool {
    crate::libm::is_sign_negative(x)
}
