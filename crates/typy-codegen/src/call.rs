//! Emits calls of the runtime and reference counting (ADR-0001).

use inkwell::IntPredicate;
use inkwell::values::{BasicValueEnum, IntValue};
use typy_tyir::{TyirType, ValueId};

use crate::error::CodegenError;
use crate::function::FunctionEmitter;
use crate::runtime::{Internal, internal_function};

impl<'ctx> FunctionEmitter<'_, '_, 'ctx> {
    /// Returns a value a runtime function gives as generated code holds it: a `bool` from the
    /// byte C's `_Bool` is, and a `float` the function may raise beside from its bits
    /// (ADR-0053).
    pub(crate) fn runtime_result(
        &self,
        result_type: TyirType,
        value: BasicValueEnum<'ctx>,
    ) -> Result<BasicValueEnum<'ctx>, CodegenError> {
        Ok(match result_type {
            TyirType::Bool => self.byte_to_bool(value.into_int_value())?.into(),
            TyirType::Float if value.is_int_value() => {
                self.builder
                    .build_bit_cast(value, self.shared.context.f64_type(), "")?
            }
            _ => value,
        })
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
            (TyirType::Int, _) => {
                return self.count_int_in_place(value.into_int_value(), increment);
            }
            (_, true) => Internal::Increment,
            (_, false) => Internal::Decrement,
        };
        let callee = internal_function(self.shared.context, self.shared.module, function);
        self.builder.build_call(callee, &[value.into()], "")?;
        if function == Internal::Decrement {
            self.close_dropped()?;
        }
        Ok(())
    }
}
