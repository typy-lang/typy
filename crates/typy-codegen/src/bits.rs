//! Emits the conversions between values and the eight bytes a list stores for them (MEM-02).

use inkwell::IntPredicate;
use inkwell::values::BasicValueEnum;
use typy_tyir::{TyirType, ValueId};

use crate::error::CodegenError;
use crate::function::FunctionEmitter;

impl<'ctx> FunctionEmitter<'_, '_, 'ctx> {
    /// Returns the eight bytes for a value: an `int` word as it is, the bits of a `float`, a
    /// `bool` as 0 or 1, or a pointer's address.
    pub(crate) fn emit_to_bits(
        &mut self,
        value: ValueId,
    ) -> Result<BasicValueEnum<'ctx>, CodegenError> {
        let word = self.shared.context.i64_type();
        let lowered = self.value(value)?;
        Ok(match self.function.value(value).ty {
            TyirType::Int => lowered,
            TyirType::Float => self.builder.build_bit_cast(lowered, word, "")?,
            TyirType::Bool => self
                .builder
                .build_int_z_extend(lowered.into_int_value(), word, "")?
                .into(),
            TyirType::Object(_) => self
                .builder
                .build_ptr_to_int(lowered.into_pointer_value(), word, "")?
                .into(),
            other => return Err(CodegenError(format!("{other} is not a list item"))),
        })
    }

    /// Returns the value of type `ty` that eight bytes hold, or `None` for `None`.
    pub(crate) fn emit_from_bits(
        &mut self,
        bits: ValueId,
        ty: TyirType,
    ) -> Result<Option<BasicValueEnum<'ctx>>, CodegenError> {
        let context = self.shared.context;
        let lowered = self.value(bits)?.into_int_value();
        Ok(Some(match ty {
            TyirType::Int | TyirType::Bits => lowered.into(),
            TyirType::Float => self
                .builder
                .build_bit_cast(lowered, context.f64_type(), "")?,
            TyirType::Bool => self
                .builder
                .build_int_compare(
                    IntPredicate::NE,
                    lowered,
                    context.i64_type().const_zero(),
                    "",
                )?
                .into(),
            TyirType::Object(_) => self
                .builder
                .build_int_to_ptr(
                    lowered,
                    context.ptr_type(inkwell::AddressSpace::default()),
                    "",
                )?
                .into(),
            TyirType::Unit => return Ok(None),
            TyirType::Tuple(_) => {
                return Err(CodegenError("a tuple is not a list item".to_owned()));
            }
        }))
    }
}
