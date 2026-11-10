//! Emits calls of the runtime and reference counting (ADR-0001).

use inkwell::IntPredicate;
use inkwell::values::{BasicValueEnum, IntValue};
use typy_tyir::{TyirType, ValueId};

use crate::error::CodegenError;
use crate::function::FunctionEmitter;
use crate::runtime::{Internal, internal_function};

impl<'ctx> FunctionEmitter<'_, '_, 'ctx> {
    pub(super) fn call_internal(
        &mut self,
        function: Internal,
        arguments: &[ValueId],
    ) -> Result<BasicValueEnum<'ctx>, CodegenError> {
        let callee = internal_function(self.shared.context, self.shared.module, function);
        let arguments = self.arguments(arguments)?;
        match self
            .builder
            .build_call(callee, &arguments, "")?
            .try_as_basic_value()
        {
            inkwell::values::ValueKind::Basic(value) => Ok(value),
            inkwell::values::ValueKind::Instruction(_) => {
                Err(CodegenError(format!("{function:?} returns no value")))
            }
        }
    }

    /// Turns the byte the runtime returns for a `bool` into an `i1`.
    pub(crate) fn byte_to_bool(
        &self,
        byte: IntValue<'ctx>,
    ) -> Result<IntValue<'ctx>, CodegenError> {
        let zero = self.shared.context.i8_type().const_zero();
        Ok(self
            .builder
            .build_int_compare(IntPredicate::NE, byte, zero, "")?)
    }

    /// Adds or drops a reference; an `int` word is counted only when it points to a large integer, which the runtime checks.
    pub(super) fn count(&mut self, value: ValueId, increment: bool) -> Result<(), CodegenError> {
        let ty = self.function.value(value).ty;
        let argument = self.value(value)?;
        self.count_value(ty, argument, increment)
    }

    /// Adds or drops a reference held by `value` of type `ty`; a tuple does so for each counted item.
    pub(crate) fn count_value(
        &mut self,
        ty: TyirType,
        value: inkwell::values::BasicValueEnum<'ctx>,
        increment: bool,
    ) -> Result<(), CodegenError> {
        let ty = match ty {
            TyirType::Optional(base) => crate::abi::storage(base),
            other => other,
        };
        let function = match (ty, increment) {
            (TyirType::Tuple(id), _) => return self.count_tuple(id, value, increment),
            (TyirType::Int, true) => Internal::IntIncrement,
            (TyirType::Int, false) => Internal::IntDecrement,
            (_, true) => Internal::Increment,
            (_, false) => Internal::Decrement,
        };
        let callee = internal_function(self.shared.context, self.shared.module, function);
        self.builder.build_call(callee, &[value.into()], "")?;
        Ok(())
    }
}
