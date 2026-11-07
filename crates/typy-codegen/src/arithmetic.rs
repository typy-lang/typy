//! Emits `int`, `float` and `bool` arithmetic; every `int` operation calls the runtime in 0.1, and inline fast paths come with the benchmarks of NFR-03.

use inkwell::values::BasicValueEnum;
use typy_tyir::{BoolOperator, FloatOperator, IntOperator, IntUnaryOperator, ValueId};

use crate::error::CodegenError;
use crate::function::FunctionEmitter;
use crate::runtime::Internal;

impl<'ctx> FunctionEmitter<'_, '_, 'ctx> {
    /// Emits an `int` operation: every one calls the runtime in 0.1.
    pub(super) fn int_binary(
        &mut self,
        operator: IntOperator,
        left: ValueId,
        right: ValueId,
    ) -> Result<BasicValueEnum<'ctx>, CodegenError> {
        let function = match operator {
            IntOperator::Add => Internal::IntAdd,
            IntOperator::Subtract => Internal::IntSubtract,
            IntOperator::Multiply => Internal::IntMultiply,
            IntOperator::BitAnd => Internal::IntAnd,
            IntOperator::BitOr => Internal::IntOr,
            IntOperator::BitXor => Internal::IntXor,
        };
        self.call_internal(function, &[left, right])
    }

    pub(super) fn int_unary(
        &mut self,
        operator: IntUnaryOperator,
        operand: ValueId,
    ) -> Result<BasicValueEnum<'ctx>, CodegenError> {
        let function = match operator {
            IntUnaryOperator::Negate => Internal::IntNegate,
            IntUnaryOperator::Invert => Internal::IntInvert,
        };
        self.call_internal(function, &[operand])
    }

    pub(super) fn float_binary(
        &mut self,
        operator: FloatOperator,
        left: ValueId,
        right: ValueId,
    ) -> Result<BasicValueEnum<'ctx>, CodegenError> {
        let left = self.value(left)?.into_float_value();
        let right = self.value(right)?.into_float_value();
        Ok(match operator {
            FloatOperator::Add => self.builder.build_float_add(left, right, "")?,
            FloatOperator::Subtract => self.builder.build_float_sub(left, right, "")?,
            FloatOperator::Multiply => self.builder.build_float_mul(left, right, "")?,
        }
        .into())
    }

    pub(super) fn bool_binary(
        &mut self,
        operator: BoolOperator,
        left: ValueId,
        right: ValueId,
    ) -> Result<BasicValueEnum<'ctx>, CodegenError> {
        let left = self.value(left)?.into_int_value();
        let right = self.value(right)?.into_int_value();
        Ok(match operator {
            BoolOperator::And => self.builder.build_and(left, right, "")?,
            BoolOperator::Or => self.builder.build_or(left, right, "")?,
            BoolOperator::Xor => self.builder.build_xor(left, right, "")?,
        }
        .into())
    }
}
