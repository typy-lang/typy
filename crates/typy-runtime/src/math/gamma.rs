//! `math.gamma` and `math.lgamma`, computed as CPython computes them rather than by the C
//! library's `tgamma` and `lgamma`: a Lanczos approximation for positive numbers and the
//! reflection formula for negative ones, over the C library's `exp`, `pow`, `log`, `sin` and
//! `cos`, so the results agree with CPython's to the last bit (LIB-12).
//!
//! Derived from CPython's `Modules/mathmodule.c` under the PSF License Agreement, which
//! THIRD-PARTY-NOTICES.md holds with a summary of the changes.
#![allow(
    clippy::float_cmp,
    reason = "CPython's algorithms compare doubles exactly, as IEEE 754 defines"
)]
#![allow(
    clippy::excessive_precision,
    reason = "the constants are written as CPython writes them"
)]

use super::c::{self, call};
use super::{domain_error_of, ok, range_error};
use crate::Raising;
use crate::libm::{absolute, copy_sign, floor, fmod, pow};

/// CPython's message for a domain error of both functions.
const POLE: &str = "expected a noninteger or positive integer, got %s";

const PI: f64 = core::f64::consts::PI;
/// The natural logarithm of pi, as CPython writes it.
const LOG_PI: f64 = 1.144_729_885_849_400_174_143_427_351_353_058_711_647;

const LANCZOS_G: f64 = 6.024_680_040_776_729_583_740_234_375;
const LANCZOS_G_MINUS_HALF: f64 = 5.524_680_040_776_729_583_740_234_375;

/// The numerator coefficients of the Lanczos sum, with N = 13.
const NUMERATOR: [f64; 13] = [
    23_531_376_880.410_759_688_572_007_674_451_636_754_734_846_804_940,
    42_919_803_642.649_098_768_957_899_047_001_988_850_926_355_848_959,
    35_711_959_237.355_668_049_440_185_451_547_166_705_960_488_635_843,
    17_921_034_426.037_209_699_919_755_754_458_931_112_671_403_265_390,
    6_039_542_586.352_028_005_064_291_644_307_297_921_069_938_842_070_8,
    1_439_720_407.311_721_673_663_223_072_794_912_393_971_548_578_677_2,
    248_874_557.862_054_156_511_460_386_413_229_423_216_321_251_278_01,
    31_426_415.585_400_194_380_614_231_628_318_205_362_874_684_987_640,
    2_876_370.628_935_372_441_225_409_051_620_849_613_599_114_537_876_8,
    186_056.265_395_223_495_040_294_989_716_045_699_282_207_842_363_28,
    8_071.672_002_365_816_210_638_002_902_272_250_613_821_851_632_502_4,
    210.824_277_751_579_345_872_509_733_920_713_362_711_669_695_802_91,
    2.506_628_274_631_000_270_164_908_177_133_837_338_626_431_079_340_8,
];

/// The denominator coefficients: those of `x * (x + 1) * ... * (x + 11)`.
const DENOMINATOR: [f64; 13] = [
    0.0,
    39_916_800.0,
    120_543_840.0,
    150_917_976.0,
    105_258_076.0,
    45_995_730.0,
    13_339_535.0,
    2_637_558.0,
    357_423.0,
    32_670.0,
    1_925.0,
    66.0,
    1.0,
];

/// The gamma function at 1 to 23, exactly.
const INTEGRAL: [f64; 23] = [
    1.0,
    1.0,
    2.0,
    6.0,
    24.0,
    120.0,
    720.0,
    5_040.0,
    40_320.0,
    362_880.0,
    3_628_800.0,
    39_916_800.0,
    479_001_600.0,
    6_227_020_800.0,
    87_178_291_200.0,
    1_307_674_368_000.0,
    20_922_789_888_000.0,
    355_687_428_096_000.0,
    6_402_373_705_728_000.0,
    121_645_100_408_832_000.0,
    2_432_902_008_176_640_000.0,
    51_090_942_171_709_440_000.0,
    1_124_000_727_777_607_680_000.0,
];

crate::export_float!(typy_rt_math_gamma = math_gamma(x: f64));

/// `math.gamma(x)`.
pub(crate) fn math_gamma(x: f64) -> Raising<f64> {
    if !x.is_finite() {
        // gamma(nan) is nan, gamma(inf) is inf, and gamma(-inf) is a domain error.
        return if x.is_nan() || x > 0.0 {
            ok(x)
        } else {
            domain_error_of(x, Some(POLE))
        };
    }
    if x == 0.0 || (x == floor(x) && x < 0.0) {
        return domain_error_of(x, Some(POLE));
    }
    if x == floor(x) && x <= 23.0 {
        #[allow(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "x is a whole number from 1 to 23"
        )]
        return ok(INTEGRAL[x as usize - 1]);
    }
    let result = tgamma(x);
    if result.is_infinite() {
        return range_error();
    }
    ok(result)
}

crate::export_float!(typy_rt_math_lgamma = math_lgamma(x: f64));

/// `math.lgamma(x)`.
pub(crate) fn math_lgamma(x: f64) -> Raising<f64> {
    if !x.is_finite() {
        // lgamma(nan) is nan, and lgamma of either infinity is inf.
        return ok(if x.is_nan() { x } else { f64::INFINITY });
    }
    if x == floor(x) && x <= 2.0 {
        return if x <= 0.0 {
            domain_error_of(x, Some(POLE))
        } else {
            ok(0.0)
        };
    }
    let absx = absolute(x);
    if absx < 1e-20 {
        return ok(-call(c::log, absx));
    }
    let mut result = call(c::log, lanczos_sum(absx)) - LANCZOS_G;
    result = multiply_add(
        absx - 0.5,
        call(c::log, absx + LANCZOS_G - 0.5) - 1.0,
        result,
    );
    if x < 0.0 {
        // The reflection formula gives the value for a negative number.
        result = LOG_PI - call(c::log, absolute(sin_pi(absx))) - call(c::log, absx) - result;
    }
    if result.is_infinite() {
        return range_error();
    }
    ok(result)
}

/// Returns gamma of a finite `x` that is not 0, not a negative integer and not an integer
/// up to 23, as CPython's `m_tgamma` does.
fn tgamma(x: f64) -> f64 {
    let absx = absolute(x);
    if absx < 1e-20 {
        return 1.0 / x;
    }
    // gamma overflows past 200 and underflows to a signed zero below -200.
    if absx > 200.0 {
        return if x < 0.0 {
            0.0 / sin_pi(x)
        } else {
            f64::INFINITY
        };
    }
    let y = absx + LANCZOS_G_MINUS_HALF;
    // The error of the sum, which corrects the result.
    let z = if absx > LANCZOS_G_MINUS_HALF {
        (y - absx) - LANCZOS_G_MINUS_HALF
    } else {
        (y - LANCZOS_G_MINUS_HALF) - absx
    };
    let z = z * LANCZOS_G / y;
    if x < 0.0 {
        let mut result = -PI / sin_pi(absx) / absx * call(c::exp, y) / lanczos_sum(absx);
        result = multiply_add(-z, result, result);
        if absx < 140.0 {
            result / pow(y, absx - 0.5)
        } else {
            let root = pow(y, absx / 2.0 - 0.25);
            result / root / root
        }
    } else {
        let mut result = lanczos_sum(absx) / call(c::exp, y);
        result = multiply_add(z, result, result);
        if absx < 140.0 {
            result * pow(y, absx - 0.5)
        } else {
            let root = pow(y, absx / 2.0 - 0.25);
            result * root * root
        }
    }
}

/// Returns the Lanczos sum of a positive `x`, as a rational function; past 5 it is taken in
/// `1 / x`, which keeps large values from overflowing.
fn lanczos_sum(x: f64) -> f64 {
    let (mut numerator, mut denominator) = (0.0, 0.0);
    if x < 5.0 {
        for index in (0..NUMERATOR.len()).rev() {
            numerator = multiply_add(numerator, x, NUMERATOR[index]);
            denominator = multiply_add(denominator, x, DENOMINATOR[index]);
        }
    } else {
        for index in 0..NUMERATOR.len() {
            numerator = numerator / x + NUMERATOR[index];
            denominator = denominator / x + DENOMINATOR[index];
        }
    }
    numerator / denominator
}

/// Returns `x * y + z` as clang compiles that expression in CPython's `mathmodule.c`: one
/// fused multiply-add on AArch64, which has the instruction, and the product rounded before
/// the sum on x86-64, whose baseline CPU has none. CPython's results then differ in the last
/// bit between the two, and TyPy gives each target's (NFR-16).
fn multiply_add(x: f64, y: f64, z: f64) -> f64 {
    if cfg!(target_arch = "aarch64") {
        // SAFETY: `fma` has no preconditions.
        unsafe { c::fma(x, y, z) }
    } else {
        x * y + z
    }
}

/// Returns `sin(pi * x)` of a finite `x`, exact at the integers, as CPython's `m_sinpi`.
fn sin_pi(x: f64) -> f64 {
    let y = fmod(absolute(x), 2.0);
    #[allow(
        clippy::cast_possible_truncation,
        reason = "2 * y is from 0 to 4, so it rounds to one of them"
    )]
    let quarter = call(c::round, 2.0 * y) as i32;
    let result = match quarter {
        0 => call(c::sin, PI * y),
        1 => call(c::cos, PI * (y - 0.5)),
        // `-sin(pi * (y - 1))` would give -0.0 for y == 1.
        2 => call(c::sin, PI * (1.0 - y)),
        3 => -call(c::cos, PI * (y - 1.5)),
        _ => call(c::sin, PI * (y - 2.0)),
    };
    copy_sign(1.0, x) * result
}
