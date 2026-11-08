//! Dispatches each TyIR operation to the module that emits it.

use inkwell::values::BasicValueEnum;
use typy_tyir::{Operation, RuntimeFunction, TyirType, ValueId};

use crate::error::CodegenError;
use crate::function::FunctionEmitter;
use crate::runtime::runtime_function;

impl<'ctx> FunctionEmitter<'_, '_, 'ctx> {
    /// Emits one operation and returns its value, if it has one.
    pub(crate) fn operation(
        &mut self,
        operation: &Operation,
    ) -> Result<Option<BasicValueEnum<'ctx>>, CodegenError> {
        Ok(match operation {
            Operation::Constant(constant) => self.constant(constant)?,
            Operation::Load(slot) => self.load(*slot)?,
            Operation::Store { slot, value } => {
                if let Some(pointer) = self.slots[slot.0 as usize] {
                    let value = self.value(*value)?;
                    self.builder.build_store(pointer, value)?;
                }
                None
            }
            Operation::IntBinary {
                operator,
                left,
                right,
            } => Some(self.int_binary(*operator, *left, *right)?),
            Operation::IntUnary { operator, operand } => Some(self.int_unary(*operator, *operand)?),
            Operation::FloatBinary {
                operator,
                left,
                right,
            } => Some(self.float_binary(*operator, *left, *right)?),
            Operation::FloatNegate(operand) => {
                let operand = self.value(*operand)?.into_float_value();
                Some(self.builder.build_float_neg(operand, "")?.into())
            }
            Operation::BoolBinary {
                operator,
                left,
                right,
            } => Some(self.bool_binary(*operator, *left, *right)?),
            Operation::BoolNot(operand) => {
                let operand = self.value(*operand)?.into_int_value();
                Some(self.builder.build_not(operand, "")?.into())
            }
            Operation::Compare {
                kind,
                comparison,
                left,
                right,
            } => Some(self.compare(*kind, *comparison, *left, *right)?.into()),
            Operation::IsTrue { kind, value } => Some(self.is_true(*kind, *value)?.into()),
            Operation::CallRuntime {
                function,
                arguments,
            } => self.call_runtime(*function, arguments)?,
            Operation::Call {
                function,
                arguments,
            } => {
                let callee = self.shared.functions[function.0 as usize];
                let arguments = self.arguments(arguments)?;
                match self
                    .builder
                    .build_call(callee, &arguments, "")?
                    .try_as_basic_value()
                {
                    inkwell::values::ValueKind::Basic(value) => Some(value),
                    inkwell::values::ValueKind::Instruction(_) => None,
                }
            }
            Operation::ToBits(value) => Some(self.emit_to_bits(*value)?),
            Operation::FromBits { bits, ty } => self.emit_from_bits(*bits, *ty)?,
            Operation::MakeTuple { ty, items } => Some(self.make_tuple(*ty, items)?),
            Operation::TupleItem { tuple, index } => self.tuple_item(*tuple, *index)?,
            Operation::Increment(value) => {
                self.count(*value, true)?;
                None
            }
            Operation::Decrement(value) => {
                self.count(*value, false)?;
                None
            }
        })
    }

    fn load(
        &mut self,
        slot: typy_tyir::SlotId,
    ) -> Result<Option<BasicValueEnum<'ctx>>, CodegenError> {
        let Some(pointer) = self.slots[slot.0 as usize] else {
            return Ok(None);
        };
        let ty = crate::abi::value_type(
            self.shared.context,
            &self.shared.tyir.tuple_types,
            self.function.slot(slot).ty,
        )
        .ok_or_else(|| CodegenError("a slot with a value has a type".to_owned()))?;
        Ok(Some(self.builder.build_load(ty, pointer, "")?))
    }

    fn call_runtime(
        &mut self,
        function: RuntimeFunction,
        arguments: &[ValueId],
    ) -> Result<Option<BasicValueEnum<'ctx>>, CodegenError> {
        let callee = runtime_function(self.shared.context, self.shared.module, function);
        let arguments = self.arguments(arguments)?;
        let call = self.builder.build_call(callee, &arguments, "")?;
        Ok(match call.try_as_basic_value() {
            inkwell::values::ValueKind::Basic(value)
                if function.signature().result == TyirType::Bool =>
            {
                Some(self.byte_to_bool(value.into_int_value())?.into())
            }
            inkwell::values::ValueKind::Basic(value) => Some(value),
            inkwell::values::ValueKind::Instruction(_) => None,
        })
    }
}
