//! Emits `int`, `float` and `bool` arithmetic; an `int` operation on inline values runs in
//! place, and the runtime computes any other (ADR-0102).

use inkwell::values::BasicValueEnum;
use typy_tyir::{BoolOperator, FloatOperator, IntOperator, IntUnaryOperator, ValueId};

use crate::error::CodegenError;
use crate::function::FunctionEmitter;

impl<'ctx> FunctionEmitter<'_, '_, 'ctx> {
    /// Emits an `int` operation of two words.
    pub(super) fn int_binary(
        &mut self,
        operator: IntOperator,
        left: ValueId,
        right: ValueId,
    ) -> Result<BasicValueEnum<'ctx>, CodegenError> {
        let left = self.value(left)?.into_int_value();
        let right = self.value(right)?.into_int_value();
        Ok(self.int_binary_in_place(operator, left, right)?.into())
    }

    /// Emits `-operand` or `~operand` of an `int` word.
    pub(super) fn int_unary(
        &mut self,
        operator: IntUnaryOperator,
        operand: ValueId,
    ) -> Result<BasicValueEnum<'ctx>, CodegenError> {
        let operand = self.value(operand)?.into_int_value();
        Ok(self.int_unary_in_place(operator, operand)?.into())
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
