//! Emits the conversions between values and the eight bytes a list stores for them (MEM-02).

use inkwell::IntPredicate;
use inkwell::values::BasicValueEnum;
use typy_tyir::{TyirType, ValueId};

use crate::error::CodegenError;
use crate::function::FunctionEmitter;

/// Returns true for a type whose items are words of a tuple's layout, which a list stores
/// in place: a tuple, or an optional one.
pub(crate) const fn holds_words(ty: TyirType) -> bool {
    matches!(
        ty,
        TyirType::Tuple(_) | TyirType::Optional(typy_tyir::OptionalBase::Tuple(_))
    )
}

/// Returns the eight bytes of `lowered`, a value of type `ty` that is one word: an `int` word
/// as it is, the bits of a `float`, a `bool` as 0 or 1, or a pointer's address, an optional
/// one as its base's storage.
pub(crate) fn word_of<'ctx>(
    builder: &inkwell::builder::Builder<'ctx>,
    word: inkwell::types::IntType<'ctx>,
    lowered: BasicValueEnum<'ctx>,
    ty: TyirType,
) -> Result<BasicValueEnum<'ctx>, CodegenError> {
    let ty = match ty {
        TyirType::Optional(typy_tyir::OptionalBase::Bool) => {
            return Ok(builder
                .build_int_z_extend(lowered.into_int_value(), word, "")?
                .into());
        }
        TyirType::Optional(base) => crate::abi::storage(base),
        other => other,
    };
    Ok(match ty {
        TyirType::Int => lowered,
        TyirType::Float => builder.build_bit_cast(lowered, word, "")?,
        TyirType::Bool => builder
            .build_int_z_extend(lowered.into_int_value(), word, "")?
            .into(),
        TyirType::Object(_) => builder
            .build_ptr_to_int(lowered.into_pointer_value(), word, "")?
            .into(),
        other => return Err(CodegenError(format!("{other} is not a list item"))),
    })
}

impl<'ctx> FunctionEmitter<'_, '_, 'ctx> {
    /// Returns the eight bytes for a value: an `int` word as it is, the bits of a `float`, a
    /// `bool` as 0 or 1, a pointer's address, 0 for `None`, or for a tuple the address of a
    /// stack slot that holds it, which the runtime copies from before the call returns.
    pub(crate) fn emit_to_bits(
        &mut self,
        value: ValueId,
    ) -> Result<BasicValueEnum<'ctx>, CodegenError> {
        let word = self.shared.context.i64_type();
        if self.function.value(value).ty == TyirType::Unit {
            return Ok(word.const_zero().into());
        }
        let lowered = self.value(value)?;
        let ty = self.function.value(value).ty;
        if holds_words(ty) {
            let slot = self.entry_alloca(lowered.get_type())?;
            self.builder.build_store(slot, lowered)?;
            return Ok(self.builder.build_ptr_to_int(slot, word, "")?.into());
        }
        word_of(&self.builder, word, lowered, ty)
    }

    /// Returns the value of type `ty` that eight bytes hold, or `None` for `None`.
    pub(crate) fn emit_from_bits(
        &mut self,
        bits: ValueId,
        ty: TyirType,
    ) -> Result<Option<BasicValueEnum<'ctx>>, CodegenError> {
        let context = self.shared.context;
        let lowered = self.value(bits)?.into_int_value();
        let ty = match ty {
            TyirType::Optional(typy_tyir::OptionalBase::Bool) => {
                return Ok(Some(
                    self.builder
                        .build_int_truncate(lowered, context.i8_type(), "")?
                        .into(),
                ));
            }
            TyirType::Optional(base) => crate::abi::storage(base),
            other => other,
        };
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
            TyirType::Optional(_) => {
                return Err(CodegenError("an optional has a storage type".to_owned()));
            }
            TyirType::Tuple(_) => {
                let struct_type =
                    crate::abi::value_type(context, &self.shared.tyir.tuple_types, ty)
                        .ok_or_else(|| CodegenError("a tuple has a type".to_owned()))?;
                let address = self.builder.build_int_to_ptr(
                    lowered,
                    context.ptr_type(inkwell::AddressSpace::default()),
                    "",
                )?;
                self.builder.build_load(struct_type, address, "")?
            }
        }))
    }

    /// Returns a stack slot of type `ty` made at the start of the function, so a slot used in a
    /// loop is made once.
    pub(crate) fn entry_alloca(
        &self,
        ty: inkwell::types::BasicTypeEnum<'ctx>,
    ) -> Result<inkwell::values::PointerValue<'ctx>, CodegenError> {
        let entry = self.blocks[0];
        let builder = self.shared.context.create_builder();
        match entry.get_first_instruction() {
            Some(first) => builder.position_before(&first),
            None => builder.position_at_end(entry),
        }
        Ok(builder.build_alloca(ty, "item")?)
    }
}
