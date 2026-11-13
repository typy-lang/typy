//! The names of the `math` module (LIB-05, ADR-0018): each constant with its value, and each
//! function with the shape of its arguments and result and the Python version that added it.

/// A function of `math`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[allow(missing_docs, reason = "each variant is the function of the same name")]
pub enum MathFunction {
    Acos,
    Acosh,
    Asin,
    Asinh,
    Atan,
    Atan2,
    Atanh,
    Cbrt,
    Ceil,
    Comb,
    Copysign,
    Cos,
    Cosh,
    Degrees,
    Erf,
    Erfc,
    Exp,
    Exp2,
    Expm1,
    Fabs,
    Factorial,
    Floor,
    Fma,
    Fmax,
    Fmin,
    Fmod,
    Frexp,
    Gcd,
    Hypot,
    Isclose,
    Isfinite,
    Isinf,
    Isnan,
    Isnormal,
    Isqrt,
    Issubnormal,
    Lcm,
    Ldexp,
    Log,
    Log10,
    Log1p,
    Log2,
    Modf,
    Nextafter,
    Perm,
    Pow,
    Radians,
    Remainder,
    Signbit,
    Sin,
    Sinh,
    Sqrt,
    Tan,
    Tanh,
    Trunc,
    Ulp,
}

/// What a function of `math` takes and gives.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MathShape {
    /// Real numbers, as many as `count`, to a `float`.
    Real(usize),
    /// A real number to an `int`: `floor`, `ceil` and `trunc`.
    Rounding,
    /// A real number to a `bool`.
    Test,
    /// `log(x)` or `log(x, base)`.
    Log,
    /// `log2(x)` and `log10(x)`.
    Logarithm,
    /// `frexp(x)`, to `(float, int)`.
    Frexp,
    /// `modf(x)`, to `(float, float)`.
    Modf,
    /// `ldexp(x, i)`.
    Ldexp,
    /// `hypot(*coordinates)`.
    Hypot,
    /// `isclose(a, b, *, rel_tol, abs_tol)`.
    Isclose,
    /// `nextafter(x, y, *, steps)`.
    Nextafter,
    /// An `int` to an `int`: `factorial` and `isqrt`.
    Integer,
    /// `comb(n, k)`.
    Comb,
    /// `perm(n)` or `perm(n, k)`.
    Perm,
    /// Any number of `int`s to an `int`: `gcd` and `lcm`.
    Fold,
}

/// The functions and their names, in name order.
const FUNCTIONS: [(&str, MathFunction); 56] = {
    use MathFunction::{
        Acos, Acosh, Asin, Asinh, Atan, Atan2, Atanh, Cbrt, Ceil, Comb, Copysign, Cos, Cosh,
        Degrees, Erf, Erfc, Exp, Exp2, Expm1, Fabs, Factorial, Floor, Fma, Fmax, Fmin, Fmod, Frexp,
        Gcd, Hypot, Isclose, Isfinite, Isinf, Isnan, Isnormal, Isqrt, Issubnormal, Lcm, Ldexp, Log,
        Log1p, Log2, Log10, Modf, Nextafter, Perm, Pow, Radians, Remainder, Signbit, Sin, Sinh,
        Sqrt, Tan, Tanh, Trunc, Ulp,
    };
    [
        ("acos", Acos),
        ("acosh", Acosh),
        ("asin", Asin),
        ("asinh", Asinh),
        ("atan", Atan),
        ("atan2", Atan2),
        ("atanh", Atanh),
        ("cbrt", Cbrt),
        ("ceil", Ceil),
        ("comb", Comb),
        ("copysign", Copysign),
        ("cos", Cos),
        ("cosh", Cosh),
        ("degrees", Degrees),
        ("erf", Erf),
        ("erfc", Erfc),
        ("exp", Exp),
        ("exp2", Exp2),
        ("expm1", Expm1),
        ("fabs", Fabs),
        ("factorial", Factorial),
        ("floor", Floor),
        ("fma", Fma),
        ("fmax", Fmax),
        ("fmin", Fmin),
        ("fmod", Fmod),
        ("frexp", Frexp),
        ("gcd", Gcd),
        ("hypot", Hypot),
        ("isclose", Isclose),
        ("isfinite", Isfinite),
        ("isinf", Isinf),
        ("isnan", Isnan),
        ("isnormal", Isnormal),
        ("isqrt", Isqrt),
        ("issubnormal", Issubnormal),
        ("lcm", Lcm),
        ("ldexp", Ldexp),
        ("log", Log),
        ("log10", Log10),
        ("log1p", Log1p),
        ("log2", Log2),
        ("modf", Modf),
        ("nextafter", Nextafter),
        ("perm", Perm),
        ("pow", Pow),
        ("radians", Radians),
        ("remainder", Remainder),
        ("signbit", Signbit),
        ("sin", Sin),
        ("sinh", Sinh),
        ("sqrt", Sqrt),
        ("tan", Tan),
        ("tanh", Tanh),
        ("trunc", Trunc),
        ("ulp", Ulp),
    ]
};

/// The constants and their values.
const CONSTANTS: [(&str, f64); 5] = [
    ("e", core::f64::consts::E),
    ("inf", f64::INFINITY),
    ("nan", f64::NAN),
    ("pi", core::f64::consts::PI),
    ("tau", core::f64::consts::TAU),
];

/// The names of `math` planned for a later release (LIB-12).
const PLANNED: [&str; 6] = ["dist", "fsum", "gamma", "lgamma", "prod", "sumprod"];

/// What a name of `math` is.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum MathName {
    /// A constant, a `float`.
    Constant(f64),
    /// A function.
    Function(MathFunction),
    /// A function planned for TyPy 0.4 (LIB-12).
    Planned,
}

/// Returns what `name` is in `math`, if it is a name of the module.
pub(crate) fn math_name(name: &str) -> Option<MathName> {
    if let Some((_, value)) = CONSTANTS.iter().find(|(known, _)| *known == name) {
        return Some(MathName::Constant(*value));
    }
    if PLANNED.contains(&name) {
        return Some(MathName::Planned);
    }
    FUNCTIONS
        .binary_search_by(|(known, _)| known.cmp(&name))
        .ok()
        .map(|index| MathName::Function(FUNCTIONS[index].1))
}

impl MathFunction {
    /// Returns the function's name in the module.
    #[must_use]
    pub fn name(self) -> &'static str {
        FUNCTIONS
            .iter()
            .find(|(_, function)| *function == self)
            .map_or("", |(name, _)| name)
    }

    /// Returns what the function takes and gives.
    #[must_use]
    pub const fn shape(self) -> MathShape {
        use MathFunction as F;
        match self {
            F::Atan2 | F::Copysign | F::Fmax | F::Fmin | F::Fmod | F::Pow | F::Remainder => {
                MathShape::Real(2)
            }
            F::Fma => MathShape::Real(3),
            F::Ceil | F::Floor | F::Trunc => MathShape::Rounding,
            F::Isfinite | F::Isinf | F::Isnan | F::Isnormal | F::Issubnormal | F::Signbit => {
                MathShape::Test
            }
            F::Log => MathShape::Log,
            F::Log2 | F::Log10 => MathShape::Logarithm,
            F::Frexp => MathShape::Frexp,
            F::Modf => MathShape::Modf,
            F::Ldexp => MathShape::Ldexp,
            F::Hypot => MathShape::Hypot,
            F::Isclose => MathShape::Isclose,
            F::Nextafter => MathShape::Nextafter,
            F::Factorial | F::Isqrt => MathShape::Integer,
            F::Comb => MathShape::Comb,
            F::Perm => MathShape::Perm,
            F::Gcd | F::Lcm => MathShape::Fold,
            _ => MathShape::Real(1),
        }
    }

    /// Returns the minor version of Python 3 that added the function: 3.13 for `fma`, 3.15
    /// for `fmax`, `fmin`, `signbit`, `isnormal` and `issubnormal`, 3.12 or earlier for the
    /// rest.
    #[must_use]
    pub const fn since(self) -> u8 {
        match self {
            Self::Fma => 13,
            Self::Fmax | Self::Fmin | Self::Signbit | Self::Isnormal | Self::Issubnormal => 15,
            _ => 12,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{CONSTANTS, FUNCTIONS, MathFunction, MathName, MathShape, math_name};

    #[test]
    fn the_names_are_sorted_and_each_function_has_one() {
        assert!(FUNCTIONS.windows(2).all(|pair| pair[0].0 < pair[1].0));
        for (name, function) in FUNCTIONS {
            assert_eq!(function.name(), name);
            assert_eq!(math_name(name), Some(MathName::Function(function)));
        }
        for (name, value) in CONSTANTS {
            let Some(MathName::Constant(found)) = math_name(name) else {
                panic!("{name} is a constant");
            };
            assert_eq!(found.to_bits(), value.to_bits(), "{name}");
        }
        assert_eq!(math_name("fsum"), Some(MathName::Planned));
        assert_eq!(math_name("square"), None);
    }

    #[test]
    fn shapes_and_versions_follow_cpython() {
        assert_eq!(MathFunction::Sqrt.shape(), MathShape::Real(1));
        assert_eq!(MathFunction::Fma.shape(), MathShape::Real(3));
        assert_eq!(MathFunction::Gcd.shape(), MathShape::Fold);
        assert_eq!(MathFunction::Fma.since(), 13);
        assert_eq!(MathFunction::Signbit.since(), 15);
        assert_eq!(MathFunction::Sqrt.since(), 12);
    }
}
