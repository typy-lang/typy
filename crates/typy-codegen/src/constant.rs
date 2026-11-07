//! Emits constants: tagged words, static large integers and static strings.

use inkwell::values::{BasicValueEnum, IntValue};
use typy_tyir::Constant;

use crate::error::CodegenError;
use crate::function::FunctionEmitter;
use crate::statics::IntEncodingValue;

impl<'ctx> FunctionEmitter<'_, '_, 'ctx> {
    pub(super) fn constant(
        &mut self,
        constant: &Constant,
    ) -> Result<Option<BasicValueEnum<'ctx>>, CodegenError> {
        let context = self.shared.context;
        Ok(match constant {
            Constant::Int(digits) => {
                Some(match self.shared.statics.int(self.shared.module, digits) {
                    IntEncodingValue::Word(word) => word.into(),
                    IntEncodingValue::Object(pointer) => self
                        .builder
                        .build_ptr_to_int(pointer, context.i64_type(), "")?
                        .into(),
                })
            }
            Constant::Float(value) => Some(context.f64_type().const_float(*value).into()),
            Constant::Bool(value) => Some(
                context
                    .bool_type()
                    .const_int(u64::from(*value), false)
                    .into(),
            ),
            Constant::Unit => None,
            Constant::Str(id) => {
                let text = self.shared.tyir.string(*id);
                Some(self.shared.statics.string(self.shared.module, text).into())
            }
        })
    }

    pub(super) fn tagged(&self, value: i64) -> IntValue<'ctx> {
        #[allow(
            clippy::cast_sign_loss,
            reason = "the tagged word is passed on bit for bit"
        )]
        let word = (value.wrapping_shl(1) | 1) as u64;
        self.shared.context.i64_type().const_int(word, true)
    }
}
