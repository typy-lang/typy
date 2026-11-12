//! Lowers calls of the functions of `math` (LIB-05, ADR-0018): each argument is converted as
//! CPython converts it, an `int` to a `float` for a real-number parameter, then the runtime
//! function of the function is called.

use typy_check::{MathFunction, MathShape, Type, TypedExpr};
use typy_diagnostics::Span;
use typy_tyir::{Constant, Operation, RuntimeFunction, TyirType, ValueId};

use crate::function::FunctionLowerer;
use crate::ownership::Lowered;

impl FunctionLowerer<'_> {
    /// Returns the result of `math.function(arguments)`; the arguments are in the order the
    /// checker put them, with a keyword not given as `None`.
    pub(crate) fn math_call(
        &mut self,
        function: MathFunction,
        arguments: &[TypedExpr],
        result: &Type,
        span: Span,
    ) -> Lowered {
        let values: Vec<Lowered> = arguments
            .iter()
            .map(|argument| self.expression(argument))
            .collect();
        let lowered = match function.shape() {
            MathShape::Real(_) => {
                let reals: Vec<ValueId> =
                    values.iter().map(|value| self.real(*value, span)).collect();
                self.call_runtime(real_function(function), reals, span)
            }
            MathShape::Test => {
                let real = self.real(values[0], span);
                self.call_runtime(real_function(function), vec![real], span)
            }
            MathShape::Rounding => self.rounded(function, values[0], span),
            MathShape::Log => {
                let natural = self.logarithm(MathFunction::Log, values[0], span);
                match values.get(1) {
                    None => natural,
                    Some(base) => {
                        let of_base = self.logarithm(MathFunction::Log, *base, span);
                        self.call_runtime(
                            RuntimeFunction::MathLogDivide,
                            vec![natural.value, of_base.value],
                            span,
                        )
                    }
                }
            }
            MathShape::Logarithm => self.logarithm(function, values[0], span),
            MathShape::Frexp | MathShape::Modf => self.split(function, values[0], result, span),
            MathShape::Ldexp => {
                let real = self.real(values[0], span);
                let exponent = self.integer(values[1], span);
                let call =
                    self.call_runtime(RuntimeFunction::MathLdexp, vec![real, exponent.value], span);
                self.release(exponent, span);
                call
            }
            MathShape::Hypot => self.hypot(&values, span),
            MathShape::Isclose => {
                let relative = self.real_or(values[2], 1e-09, span);
                let absolute = self.real_or(values[3], 0.0, span);
                let (a, b) = (self.real(values[0], span), self.real(values[1], span));
                self.call_runtime(
                    RuntimeFunction::MathIsclose,
                    vec![a, b, relative, absolute],
                    span,
                )
            }
            MathShape::Nextafter => {
                let (x, y) = (self.real(values[0], span), self.real(values[1], span));
                let steps = self.integer_or_one(values[2], span);
                self.call_runtime(
                    RuntimeFunction::MathNextafter,
                    vec![x, y, steps.value],
                    span,
                )
            }
            MathShape::Integer => {
                let integer = self.integer(values[0], span);
                let runtime = if function == MathFunction::Factorial {
                    RuntimeFunction::MathFactorial
                } else {
                    RuntimeFunction::MathIsqrt
                };
                let call = self.call_runtime(runtime, vec![integer.value], span);
                self.release(integer, span);
                call
            }
            MathShape::Comb => {
                let (n, k) = (self.integer(values[0], span), self.integer(values[1], span));
                let call =
                    self.call_runtime(RuntimeFunction::MathComb, vec![n.value, k.value], span);
                self.release(n, span);
                self.release(k, span);
                call
            }
            MathShape::Perm => self.permutations(&values, span),
            MathShape::Fold => self.fold(function, &values, span),
        };
        for value in values {
            self.release(value, span);
        }
        lowered
    }

    /// Returns an `int` argument as an `int`: borrowed when it is one already, since the
    /// argument itself is released after the call, and otherwise the converted value.
    fn integer(&mut self, value: Lowered, span: Span) -> Lowered {
        let converted = self.int_of(value, span);
        if converted.value == value.value {
            return Self::borrowed(value.value, TyirType::Int);
        }
        converted
    }

    /// Returns a real-number argument as a `float`, which raises `OverflowError` for an
    /// `int` too large for one.
    fn real(&mut self, value: Lowered, span: Span) -> ValueId {
        self.float_of(value, span).value
    }

    /// Returns a keyword argument as a `float`, or `default` when it was not given.
    fn real_or(&mut self, value: Lowered, default: f64, span: Span) -> ValueId {
        if value.ty == TyirType::Unit {
            return self.constant(Constant::Float(default), span);
        }
        self.real(value, span)
    }

    /// Returns an `int` argument that may be `None`, with 1 for `None`: the steps of
    /// `nextafter`.
    fn integer_or_one(&mut self, value: Lowered, span: Span) -> Lowered {
        let one = |lowerer: &mut Self| {
            let value = lowerer.constant(Constant::Int("1".to_owned()), span);
            Self::borrowed(value, TyirType::Int)
        };
        match value.ty {
            TyirType::Unit => one(self),
            TyirType::Optional(_) => {
                self.optional_branch(value, TyirType::Int, span, one, |lowerer, held| {
                    lowerer.integer(held, span)
                })
            }
            _ => self.integer(value, span),
        }
    }

    /// Returns `floor`, `ceil` or `trunc` of a number: an `int` itself, and a `float` rounded
    /// then made an `int`, which raises for an infinity or a NaN.
    fn rounded(&mut self, function: MathFunction, value: Lowered, span: Span) -> Lowered {
        if value.ty != TyirType::Float {
            let integer = self.integer(value, span);
            let value = self.take_reference(integer, span);
            return self.produced(value, TyirType::Int);
        }
        let rounded = match function {
            MathFunction::Floor => {
                self.call_runtime(RuntimeFunction::MathFloor, vec![value.value], span)
                    .value
            }
            MathFunction::Ceil => {
                self.call_runtime(RuntimeFunction::MathCeil, vec![value.value], span)
                    .value
            }
            _ => value.value,
        };
        self.call_runtime(RuntimeFunction::FloatToInt, vec![rounded], span)
    }

    /// Returns a logarithm of a number: an `int`, of any size, by its own path, as CPython
    /// takes it.
    fn logarithm(&mut self, function: MathFunction, value: Lowered, span: Span) -> Lowered {
        let (of_int, of_float) = match function {
            MathFunction::Log2 => (RuntimeFunction::MathLog2Int, RuntimeFunction::MathLog2Float),
            MathFunction::Log10 => (
                RuntimeFunction::MathLog10Int,
                RuntimeFunction::MathLog10Float,
            ),
            _ => (RuntimeFunction::MathLogInt, RuntimeFunction::MathLogFloat),
        };
        if value.ty == TyirType::Float {
            return self.call_runtime(of_float, vec![value.value], span);
        }
        let integer = self.integer(value, span);
        let call = self.call_runtime(of_int, vec![integer.value], span);
        self.release(integer, span);
        call
    }

    /// Returns the tuple `frexp` or `modf` gives.
    fn split(
        &mut self,
        function: MathFunction,
        value: Lowered,
        result: &Type,
        span: Span,
    ) -> Lowered {
        let real = self.real(value, span);
        let (first, second) = if function == MathFunction::Frexp {
            (
                RuntimeFunction::MathFrexpMantissa,
                RuntimeFunction::MathFrexpExponent,
            )
        } else {
            (
                RuntimeFunction::MathModfFraction,
                RuntimeFunction::MathModfIntegral,
            )
        };
        let first = self.call_runtime(first, vec![real], span).value;
        let second = self.call_runtime(second, vec![real], span).value;
        let ty = self.lowered_type(result);
        let (id, _) = self.item_types(ty);
        let value = self.builder.push_value(
            Operation::MakeTuple {
                ty: id,
                items: vec![first, second],
            },
            ty,
            span,
        );
        self.produced(value, ty)
    }

    /// Returns `hypot` of the coordinates, given to the runtime as a new list of `float`s.
    fn hypot(&mut self, values: &[Lowered], span: Span) -> Lowered {
        let list = self.list_display(&[], TyirType::Float, span);
        for value in values {
            let real = self.real(*value, span);
            let real = Self::borrowed(real, TyirType::Float);
            let bits = self.bits(real, span);
            self.call_runtime(RuntimeFunction::ListAppend, vec![list.value, bits], span);
        }
        let result = self.call_runtime(RuntimeFunction::MathHypot, vec![list.value], span);
        self.release(list, span);
        result
    }

    /// Returns `perm(n)`, or `perm(n, k)` where `k` may be `None`, which is `perm(n)`.
    fn permutations(&mut self, values: &[Lowered], span: Span) -> Lowered {
        let n = self.integer(values[0], span);
        let all = |lowerer: &mut Self| {
            lowerer.call_runtime(RuntimeFunction::MathPermAll, vec![n.value], span)
        };
        let result = match values.get(1).copied() {
            None => all(self),
            Some(k) if k.ty == TyirType::Unit => all(self),
            Some(k) if matches!(k.ty, TyirType::Optional(_)) => {
                self.optional_branch(k, TyirType::Int, span, all, |lowerer, held| {
                    let k = lowerer.integer(held, span);
                    let call = lowerer.call_runtime(
                        RuntimeFunction::MathPerm,
                        vec![n.value, k.value],
                        span,
                    );
                    lowerer.release(k, span);
                    call
                })
            }
            Some(k) => {
                let k = self.integer(k, span);
                let call =
                    self.call_runtime(RuntimeFunction::MathPerm, vec![n.value, k.value], span);
                self.release(k, span);
                call
            }
        };
        self.release(n, span);
        result
    }

    /// Returns `gcd` or `lcm` of any number of `int`s, folded from 0 or 1, the result of
    /// each with no argument.
    fn fold(&mut self, function: MathFunction, values: &[Lowered], span: Span) -> Lowered {
        let (start, runtime) = if function == MathFunction::Gcd {
            ("0", RuntimeFunction::MathGcd)
        } else {
            ("1", RuntimeFunction::MathLcm)
        };
        let start = self.constant(Constant::Int(start.to_owned()), span);
        let mut total = Self::borrowed(start, TyirType::Int);
        for value in values {
            let integer = self.integer(*value, span);
            let next = self.call_runtime(runtime, vec![total.value, integer.value], span);
            self.release(integer, span);
            self.release(total, span);
            total = next;
        }
        // The result holds its own reference, moved or counted as `total` holds one or not.
        let value = self.take_reference(total, span);
        self.produced(value, TyirType::Int)
    }
}

/// Returns the runtime function of a function of one, two or three `float`s, or of a test.
fn real_function(function: MathFunction) -> RuntimeFunction {
    use MathFunction as F;
    use RuntimeFunction as R;
    match function {
        F::Acos => R::MathAcos,
        F::Acosh => R::MathAcosh,
        F::Asin => R::MathAsin,
        F::Asinh => R::MathAsinh,
        F::Atan => R::MathAtan,
        F::Atan2 => R::MathAtan2,
        F::Atanh => R::MathAtanh,
        F::Cbrt => R::MathCbrt,
        F::Copysign => R::MathCopysign,
        F::Cos => R::MathCos,
        F::Cosh => R::MathCosh,
        F::Degrees => R::MathDegrees,
        F::Erf => R::MathErf,
        F::Erfc => R::MathErfc,
        F::Exp => R::MathExp,
        F::Exp2 => R::MathExp2,
        F::Expm1 => R::MathExpm1,
        F::Fabs => R::MathFabs,
        F::Fma => R::MathFma,
        F::Fmax => R::MathFmax,
        F::Fmin => R::MathFmin,
        F::Fmod => R::MathFmod,
        F::Isfinite => R::MathIsfinite,
        F::Isinf => R::MathIsinf,
        F::Isnan => R::MathIsnan,
        F::Isnormal => R::MathIsnormal,
        F::Issubnormal => R::MathIssubnormal,
        F::Log1p => R::MathLog1p,
        F::Pow => R::MathPow,
        F::Radians => R::MathRadians,
        F::Remainder => R::MathRemainder,
        F::Signbit => R::MathSignbit,
        F::Sin => R::MathSin,
        F::Sinh => R::MathSinh,
        F::Sqrt => R::MathSqrt,
        F::Tan => R::MathTan,
        F::Tanh => R::MathTanh,
        _ => R::MathUlp,
    }
}
