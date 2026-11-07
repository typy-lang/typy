//! Emits comparisons and truth tests (EXP-06, EXP-10).

use inkwell::values::IntValue;
use inkwell::{FloatPredicate, IntPredicate};
use typy_tyir::{Comparison, RuntimeFunction, ScalarKind, ValueId};

use crate::error::CodegenError;
use crate::function::FunctionEmitter;
use crate::runtime::{Internal, internal_function, runtime_function};

impl<'ctx> FunctionEmitter<'_, '_, 'ctx> {
    pub(super) fn compare(
        &mut self,
        kind: ScalarKind,
        comparison: Comparison,
        left: ValueId,
        right: ValueId,
    ) -> Result<IntValue<'ctx>, CodegenError> {
        let left_value = self.value(left)?;
        let right_value = self.value(right)?;
        let builder = &self.builder;
        Ok(match kind {
            ScalarKind::Int => {
                let callee = internal_function(
                    self.shared.context,
                    self.shared.module,
                    Internal::IntCompare,
                );
                let code = self.tagged(comparison.code());
                let byte = builder
                    .build_call(
                        callee,
                        &[left_value.into(), right_value.into(), code.into()],
                        "",
                    )?
                    .try_as_basic_value();
                match byte {
                    inkwell::values::ValueKind::Basic(value) => {
                        self.byte_to_bool(value.into_int_value())?
                    }
                    inkwell::values::ValueKind::Instruction(_) => {
                        return Err(CodegenError("int_compare returns a byte".to_owned()));
                    }
                }
            }
            ScalarKind::Str => {
                let callee = runtime_function(
                    self.shared.context,
                    self.shared.module,
                    RuntimeFunction::StrCompare,
                );
                let code = self.tagged(comparison.code());
                match builder
                    .build_call(
                        callee,
                        &[left_value.into(), right_value.into(), code.into()],
                        "",
                    )?
                    .try_as_basic_value()
                {
                    inkwell::values::ValueKind::Basic(value) => {
                        self.byte_to_bool(value.into_int_value())?
                    }
                    inkwell::values::ValueKind::Instruction(_) => {
                        return Err(CodegenError("str_compare returns a byte".to_owned()));
                    }
                }
            }
            ScalarKind::Float => {
                // NaN compares unequal to everything, as in Python, so `!=` is the unordered comparison.
                let predicate = match comparison {
                    Comparison::Equal => FloatPredicate::OEQ,
                    Comparison::NotEqual => FloatPredicate::UNE,
                    Comparison::Less => FloatPredicate::OLT,
                    Comparison::LessEqual => FloatPredicate::OLE,
                    Comparison::Greater => FloatPredicate::OGT,
                    Comparison::GreaterEqual => FloatPredicate::OGE,
                };
                builder.build_float_compare(
                    predicate,
                    left_value.into_float_value(),
                    right_value.into_float_value(),
                    "",
                )?
            }
            ScalarKind::Bool => {
                let predicate = match comparison {
                    Comparison::Equal => IntPredicate::EQ,
                    Comparison::NotEqual => IntPredicate::NE,
                    Comparison::Less => IntPredicate::ULT,
                    Comparison::LessEqual => IntPredicate::ULE,
                    Comparison::Greater => IntPredicate::UGT,
                    Comparison::GreaterEqual => IntPredicate::UGE,
                };
                builder.build_int_compare(
                    predicate,
                    left_value.into_int_value(),
                    right_value.into_int_value(),
                    "",
                )?
            }
        })
    }

    pub(super) fn is_true(
        &mut self,
        kind: ScalarKind,
        value: ValueId,
    ) -> Result<IntValue<'ctx>, CodegenError> {
        let value = self.value(value)?;
        let context = self.shared.context;
        Ok(match kind {
            // Zero is the tagged word 1; a large integer is never zero.
            ScalarKind::Int => self.builder.build_int_compare(
                IntPredicate::NE,
                value.into_int_value(),
                self.tagged(0),
                "",
            )?,
            ScalarKind::Float => self.builder.build_float_compare(
                FloatPredicate::UNE,
                value.into_float_value(),
                context.f64_type().const_zero(),
                "",
            )?,
            ScalarKind::Bool => value.into_int_value(),
            ScalarKind::Str => {
                let callee =
                    runtime_function(context, self.shared.module, RuntimeFunction::StrLength);
                match self
                    .builder
                    .build_call(callee, &[value.into()], "")?
                    .try_as_basic_value()
                {
                    inkwell::values::ValueKind::Basic(length) => self.builder.build_int_compare(
                        IntPredicate::NE,
                        length.into_int_value(),
                        self.tagged(0),
                        "",
                    )?,
                    inkwell::values::ValueKind::Instruction(_) => {
                        return Err(CodegenError("str_length returns an int".to_owned()));
                    }
                }
            }
        })
    }
}
