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
            Constant::Bytes(id) => {
                let text = self.shared.tyir.string(*id);
                Some(self.shared.statics.bytes(self.shared.module, text).into())
            }
            Constant::PlaceholderList | Constant::PlaceholderSet => {
                let name = if *constant == Constant::PlaceholderList {
                    "typy_rt_placeholder_list"
                } else {
                    "typy_rt_placeholder_set"
                };
                let global = self.shared.module.get_global(name).unwrap_or_else(|| {
                    let global = self.shared.module.add_global(context.i8_type(), None, name);
                    global.set_linkage(inkwell::module::Linkage::External);
                    global
                });
                Some(global.as_pointer_value().into())
            }
            Constant::TupleDescriptor(id) => {
                let descriptor = crate::descriptor::tuple_descriptor(self.shared, *id)?;
                Some(
                    self.builder
                        .build_ptr_to_int(descriptor, context.i64_type(), "")?
                        .into(),
                )
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
