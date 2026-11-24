//! Writes `//` and `%` of inline `int` words, `float()` of one, and `/` of floats in place, and
//! calls the runtime only for a large integer, a divisor of zero, or the one quotient outside
//! the inline range, `-2**62 // -1` (ADR-0107).
//!
//! Python rounds a quotient toward negative infinity, while LLVM's `sdiv` rounds toward zero:
//! where the remainder is not zero and its sign differs from the divisor's, the quotient is one
//! less and the remainder one divisor more.

use inkwell::values::{BasicValueEnum, IntValue};
use inkwell::{FloatPredicate, IntPredicate};
use typy_tyir::{RuntimeFunction, ValueId};

use crate::error::CodegenError;
use crate::function::FunctionEmitter;
use crate::in_place_call::Raised;

impl<'ctx> FunctionEmitter<'_, '_, 'ctx> {
    /// Returns `left // right` for `IntFloorDivide` or `left % right` for `IntModulo` of two
    /// `int` words.
    pub(crate) fn int_division_in_place(
        &mut self,
        function: RuntimeFunction,
        arguments: &[ValueId],
    ) -> Result<Raised<'ctx>, CodegenError> {
        let word = self.shared.context.i64_type();
        let left = self.value(arguments[0])?.into_int_value();
        let right = self.value(arguments[1])?.into_int_value();
        let inline = self.both_inline(left, right)?;
        // The inline word of 0 is 1, of -1 is -1, and of -2**62 is the smallest word plus one.
        let zero = word.const_int(1, false);
        let nonzero = self
            .builder
            .build_int_compare(IntPredicate::NE, right, zero, "")?;
        let smallest = word.const_int((i64::MIN + 1).cast_unsigned(), true);
        let minus_one = word.const_all_ones();
        let left_smallest = self
            .builder
            .build_int_compare(IntPredicate::EQ, left, smallest, "")?;
        let right_minus_one =
            self.builder
                .build_int_compare(IntPredicate::EQ, right, minus_one, "")?;
        let outside = self.builder.build_and(left_smallest, right_minus_one, "")?;
        let fits = self.builder.build_not(outside, "")?;
        let fast = self.builder.build_and(inline, nonzero, "")?;
        let fast = self.builder.build_and(fast, fits, "")?;
        self.in_place_or_call(fast, function, arguments, |this| {
            let one = word.const_int(1, false);
            let a = this.builder.build_right_shift(left, one, true, "")?;
            let b = this.builder.build_right_shift(right, one, true, "")?;
            let quotient = this.builder.build_int_signed_div(a, b, "")?;
            let remainder = this.builder.build_int_signed_rem(a, b, "")?;
            let inexact = this.builder.build_int_compare(
                IntPredicate::NE,
                remainder,
                word.const_zero(),
                "",
            )?;
            let signs = this.builder.build_xor(remainder, b, "")?;
            let differ =
                this.builder
                    .build_int_compare(IntPredicate::SLT, signs, word.const_zero(), "")?;
            let adjust = this.builder.build_and(inexact, differ, "")?;
            let value = if function == RuntimeFunction::IntFloorDivide {
                let down = this.builder.build_int_z_extend(adjust, word, "")?;
                this.builder.build_int_sub(quotient, down, "")?
            } else {
                let up = this
                    .builder
                    .build_select(adjust, b, word.const_zero(), "")?
                    .into_int_value();
                this.builder.build_int_add(remainder, up, "")?
            };
            Ok(this.inline_word(value)?.into())
        })
    }

    /// Returns `float(value)` of an `int` word: an inline value converts in place, rounded to
    /// nearest with ties to even as the runtime rounds it.
    pub(crate) fn int_to_float_in_place(
        &mut self,
        arguments: &[ValueId],
    ) -> Result<Raised<'ctx>, CodegenError> {
        let value = self.value(arguments[0])?.into_int_value();
        let fast = self.is_inline(value)?;
        self.in_place_or_call(fast, RuntimeFunction::IntToFloat, arguments, |this| {
            let context = this.shared.context;
            let number = this.builder.build_right_shift(
                value,
                context.i64_type().const_int(1, false),
                true,
                "",
            )?;
            Ok(this
                .builder
                .build_signed_int_to_float(number, context.f64_type(), "")?
                .into())
        })
    }

    /// Returns `left / right` of two floats; only a divisor of zero, either sign, reaches the
    /// runtime, which raises `ZeroDivisionError`.
    pub(crate) fn float_divide_in_place(
        &mut self,
        arguments: &[ValueId],
    ) -> Result<Raised<'ctx>, CodegenError> {
        let left = self.value(arguments[0])?.into_float_value();
        let right = self.value(arguments[1])?.into_float_value();
        let zero = self.shared.context.f64_type().const_zero();
        // Unordered or not equal: a NaN divisor divides in place, as the runtime divides it.
        let fast = self
            .builder
            .build_float_compare(FloatPredicate::UNE, right, zero, "")?;
        self.in_place_or_call(fast, RuntimeFunction::FloatDivide, arguments, |this| {
            let quotient: BasicValueEnum<'ctx> =
                this.builder.build_float_div(left, right, "")?.into();
            Ok(quotient)
        })
    }

    /// Returns the inline word of `value`, which is inside the inline range: `2 * value + 1`.
    fn inline_word(&self, value: IntValue<'ctx>) -> Result<IntValue<'ctx>, CodegenError> {
        let word = self.shared.context.i64_type();
        let doubled = self
            .builder
            .build_left_shift(value, word.const_int(1, false), "")?;
        Ok(self
            .builder
            .build_or(doubled, word.const_int(1, false), "")?)
    }
}
