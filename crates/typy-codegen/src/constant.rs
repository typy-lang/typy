//! Emits constants: tagged words, static large integers and static strings.

use inkwell::values::{BasicValueEnum, IntValue};
use typy_tyir::Constant;

use crate::error::CodegenError;
use crate::function::FunctionEmitter;
use crate::statics::IntEncodingValue;

impl<'ctx> FunctionEmitter<'_, '_, 'ctx> {
    /// Returns the immortal static object of a type alias (SMT-12) or a typing value
    /// (TYP-18), emitted the first time it is used.
    fn typing_object(&mut self, constant: &Constant) -> inkwell::values::PointerValue<'ctx> {
        let module = self.shared.module;
        match constant {
            Constant::TypeAlias(index) => {
                let alias = &self.shared.tyir.type_aliases[*index as usize];
                self.shared
                    .statics
                    .type_alias_object(module, *index, &alias.name, &alias.module)
            }
            Constant::TypingValue(index) => {
                let value = &self.shared.tyir.typing_values[*index as usize];
                self.shared
                    .statics
                    .typing_value_object(module, *index, &value.text, &value.key)
            }
            _ => unreachable!("only an alias or a typing value is emitted here"),
        }
    }

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
            Constant::NoneOf(base) => Some(self.none_of(*base)?),
            Constant::TypeObject(
                reference @ (typy_tyir::TypeObjectRef::Runtime(_)
                | typy_tyir::TypeObjectRef::Exception(_)),
            ) => {
                let symbol = reference
                    .runtime_symbol()
                    .ok_or_else(|| CodegenError("a runtime type object has a symbol".to_owned()))?;
                Some(crate::class::runtime_global(self.shared, &symbol).into())
            }
            Constant::TypeObject(typy_tyir::TypeObjectRef::Class(index)) => {
                Some(crate::class::type_object(self.shared, *index)?.into())
            }
            Constant::Function(_)
            | Constant::MethodDescriptor(_)
            | Constant::TupleDescriptor(_)
            | Constant::CDataClass(_) => Some(self.emitted(constant)?),
            Constant::TypeAlias(_) | Constant::TypingValue(_) => {
                Some(self.typing_object(constant).into())
            }
            Constant::Module(index) => {
                let object = &self.shared.tyir.modules[*index as usize];
                Some(
                    self.shared
                        .statics
                        .module_object(self.shared.module, *index, &object.name, &object.path)
                        .into(),
                )
            }
            Constant::NoInstance(_) => Some(
                self.shared
                    .context
                    .ptr_type(inkwell::AddressSpace::default())
                    .const_null()
                    .into(),
            ),
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
            Constant::StrPoints(index) => {
                let points = &self.shared.tyir.str_points[*index as usize];
                Some(
                    self.shared
                        .statics
                        .str_points(self.shared.module, *index, points)
                        .into(),
                )
            }
            Constant::Bytes(id) => {
                let text = self.shared.tyir.string(*id);
                Some(self.shared.statics.bytes(self.shared.module, text).into())
            }
            Constant::PlaceholderList | Constant::PlaceholderSet | Constant::PlaceholderDict => {
                let name = match constant {
                    Constant::PlaceholderList => "typy_rt_placeholder_list",
                    Constant::PlaceholderSet => "typy_rt_placeholder_set",
                    _ => "typy_rt_placeholder_dict",
                };
                let global = self.shared.module.get_global(name).unwrap_or_else(|| {
                    let global = self.shared.module.add_global(context.i8_type(), None, name);
                    global.set_linkage(inkwell::module::Linkage::External);
                    global
                });
                Some(global.as_pointer_value().into())
            }
        })
    }

    /// Returns a constant that is a static emitted the first time it is used: a function
    /// object, or a descriptor or a class, an address the runtime takes as a word.
    fn emitted(&mut self, constant: &Constant) -> Result<BasicValueEnum<'ctx>, CodegenError> {
        match constant {
            Constant::Function(id) => {
                Ok(crate::function_object::function_object(self.shared, *id)?.into())
            }
            Constant::MethodDescriptor(id) => {
                let descriptor = crate::function_object::method_descriptor(self.shared, *id)?;
                self.address_word(descriptor)
            }
            Constant::TupleDescriptor(id) => {
                let descriptor = crate::descriptor::tuple_descriptor(self.shared, *id)?;
                self.address_word(descriptor)
            }
            Constant::CDataClass(index) => {
                let class = crate::cdata::cdata_class(self.shared, *index)?;
                self.address_word(class)
            }
            other => Err(CodegenError(format!("{other} is not an emitted static"))),
        }
    }

    /// Returns the address of a static as a word.
    fn address_word(
        &self,
        address: inkwell::values::PointerValue<'ctx>,
    ) -> Result<BasicValueEnum<'ctx>, CodegenError> {
        let word = self.shared.context.i64_type();
        Ok(self.builder.build_ptr_to_int(address, word, "")?.into())
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
