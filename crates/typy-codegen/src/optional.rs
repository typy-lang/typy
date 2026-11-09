//! Emits optional values: `None` held in a value each type never takes (ADR-0011).

use inkwell::IntPredicate;
use inkwell::values::BasicValueEnum;
use typy_tyir::{OptionalBase, TyirType, ValueId};

use crate::error::CodegenError;
use crate::function::FunctionEmitter;

/// The bits of `None` of a `float | None`, a signalling NaN no float operation produces.
pub(crate) const NONE_FLOAT: u64 = 0x7ff4_0000_0000_0001;

/// The byte of `None` of a `bool | None`.
const NONE_BOOL: u64 = 2;

impl<'ctx> FunctionEmitter<'_, '_, 'ctx> {
    /// Returns the optional base of a value's type.
    fn base_of(&self, value: ValueId) -> Result<OptionalBase, CodegenError> {
        match self.function.value(value).ty {
            TyirType::Optional(base) => Ok(base),
            other => Err(CodegenError(format!("{other} is not optional"))),
        }
    }

    /// Returns `None` of an optional type.
    pub(crate) fn none_of(
        &mut self,
        base: OptionalBase,
    ) -> Result<BasicValueEnum<'ctx>, CodegenError> {
        let context = self.shared.context;
        Ok(match base {
            OptionalBase::Int => context.i64_type().const_zero().into(),
            OptionalBase::Float => self.builder.build_bit_cast(
                context.i64_type().const_int(NONE_FLOAT, false),
                context.f64_type(),
                "",
            )?,
            OptionalBase::Bool => context.i8_type().const_int(NONE_BOOL, false).into(),
            OptionalBase::Object(_) => context
                .ptr_type(inkwell::AddressSpace::default())
                .const_null()
                .into(),
            OptionalBase::Tuple(pair) => crate::abi::value_type(
                context,
                &self.shared.tyir.tuple_types,
                TyirType::Tuple(pair),
            )
            .ok_or_else(|| CodegenError("a pair has a type".to_owned()))?
            .const_zero(),
        })
    }

    /// Returns an optional value of type `ty` that holds `value`: the value itself, a `bool`
    /// widened to a byte, or a tuple with its presence flag set.
    pub(crate) fn wrap(
        &mut self,
        value: ValueId,
        ty: TyirType,
    ) -> Result<BasicValueEnum<'ctx>, CodegenError> {
        let lowered = self.value(value)?;
        let TyirType::Optional(base) = ty else {
            return Err(CodegenError(format!("{ty} is not optional")));
        };
        Ok(match base {
            OptionalBase::Bool => self
                .builder
                .build_int_z_extend(lowered.into_int_value(), self.shared.context.i8_type(), "")?
                .into(),
            OptionalBase::Tuple(pair) => {
                let pair_type = crate::abi::value_type(
                    self.shared.context,
                    &self.shared.tyir.tuple_types,
                    TyirType::Tuple(pair),
                )
                .ok_or_else(|| CodegenError("a pair has a type".to_owned()))?
                .into_struct_type();
                let present = self.shared.context.bool_type().const_int(1, false);
                let flagged =
                    self.builder
                        .build_insert_value(pair_type.get_undef(), present, 0, "")?;
                self.builder
                    .build_insert_value(flagged, lowered, 1, "")?
                    .into_struct_value()
                    .into()
            }
            OptionalBase::Int | OptionalBase::Float | OptionalBase::Object(_) => lowered,
        })
    }

    /// Returns the value an optional holds, which is not `None`.
    pub(crate) fn unwrap(&mut self, value: ValueId) -> Result<BasicValueEnum<'ctx>, CodegenError> {
        let base = self.base_of(value)?;
        let lowered = self.value(value)?;
        Ok(match base {
            OptionalBase::Bool => self
                .builder
                .build_int_truncate(
                    lowered.into_int_value(),
                    self.shared.context.bool_type(),
                    "",
                )?
                .into(),
            OptionalBase::Tuple(_) => {
                self.builder
                    .build_extract_value(lowered.into_struct_value(), 1, "")?
            }
            OptionalBase::Int | OptionalBase::Float | OptionalBase::Object(_) => lowered,
        })
    }

    /// Returns whether an optional value is `None`.
    pub(crate) fn is_none(&mut self, value: ValueId) -> Result<BasicValueEnum<'ctx>, CodegenError> {
        let base = self.base_of(value)?;
        let lowered = self.value(value)?;
        let context = self.shared.context;
        let equal = |emitter: &mut Self,
                     word: inkwell::values::IntValue<'ctx>,
                     none: inkwell::values::IntValue<'ctx>| {
            emitter
                .builder
                .build_int_compare(IntPredicate::EQ, word, none, "")
        };
        Ok(match base {
            OptionalBase::Int => {
                let none = context.i64_type().const_zero();
                equal(self, lowered.into_int_value(), none)?.into()
            }
            OptionalBase::Float => {
                let bits = self
                    .builder
                    .build_bit_cast(lowered, context.i64_type(), "")?
                    .into_int_value();
                equal(self, bits, context.i64_type().const_int(NONE_FLOAT, false))?.into()
            }
            OptionalBase::Bool => {
                let none = context.i8_type().const_int(NONE_BOOL, false);
                equal(self, lowered.into_int_value(), none)?.into()
            }
            OptionalBase::Object(_) => self
                .builder
                .build_is_null(lowered.into_pointer_value(), "")?
                .into(),
            OptionalBase::Tuple(_) => {
                let present = self
                    .builder
                    .build_extract_value(lowered.into_struct_value(), 0, "")?
                    .into_int_value();
                self.builder.build_not(present, "")?.into()
            }
        })
    }
}
