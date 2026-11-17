//! Emits a call of a C function a `ctypes` declaration describes: its declaration with C
//! types, each argument converted to its C type, and the result converted back, as `ctypes`
//! converts them (FFI-02, FFI-03, ADR-0047).

use inkwell::AddressSpace;
use inkwell::context::Context;
use inkwell::module::Linkage;
use inkwell::types::{BasicMetadataTypeEnum, BasicType, BasicTypeEnum, FunctionType};
use inkwell::values::{BasicMetadataValueEnum, BasicValueEnum, CallSiteValue};
use typy_tyir::{CType, TyirType, ValueId};

use crate::class::runtime_symbol;
use crate::error::CodegenError;
use crate::function::FunctionEmitter;

/// Returns the LLVM type a C type passes as: a `bool` as a byte.
pub(crate) fn c_type(context: &Context, ty: CType) -> BasicTypeEnum<'_> {
    match ty {
        CType::Int8 | CType::UInt8 | CType::Bool => context.i8_type().into(),
        CType::Int16 | CType::UInt16 => context.i16_type().into(),
        CType::Int32 | CType::UInt32 => context.i32_type().into(),
        CType::Int64 | CType::UInt64 => context.i64_type().into(),
        CType::Float => context.f32_type().into(),
        CType::Double => context.f64_type().into(),
        CType::CharPointer | CType::VoidPointer | CType::Pointer(_) => {
            context.ptr_type(AddressSpace::default()).into()
        }
    }
}

/// Returns the LLVM type an argument of C type `ty` passes as: an integer or a `bool` as a
/// whole 64-bit word, widened by its own sign or by zeros, as `libffi` passes it for
/// CPython's `ctypes`, so a function whose C type is wider than its declaration reads what
/// CPython's would.
fn argument_type(context: &Context, ty: CType) -> BasicTypeEnum<'_> {
    if ty.integer().is_some() || ty == CType::Bool {
        return context.i64_type().into();
    }
    c_type(context, ty)
}

impl<'ctx> FunctionEmitter<'_, '_, 'ctx> {
    /// Calls the C function at `function` with `arguments`, and returns its result as its
    /// TyPy value, or `None` for `void`.
    pub(crate) fn foreign_call(
        &mut self,
        function: u32,
        arguments: &[ValueId],
    ) -> Result<Option<BasicValueEnum<'ctx>>, CodegenError> {
        let foreign = self
            .shared
            .tyir
            .foreign_functions
            .get(function as usize)
            .ok_or_else(|| CodegenError(format!("foreign#{function} does not exist")))?
            .clone();
        let context = self.shared.context;
        let parameters: Vec<BasicMetadataTypeEnum<'ctx>> = foreign
            .parameters
            .iter()
            .map(|ty| argument_type(context, *ty).into())
            .collect();
        let ty: FunctionType<'ctx> = match foreign.result {
            Some(result) => c_type(context, result).fn_type(&parameters, false),
            None => context.void_type().fn_type(&parameters, false),
        };
        let callee = self
            .shared
            .module
            .get_function(&foreign.name)
            .unwrap_or_else(|| {
                self.shared
                    .module
                    .add_function(&foreign.name, ty, Some(Linkage::External))
            });
        let mut converted: Vec<BasicMetadataValueEnum<'ctx>> = Vec::new();
        for (value, parameter) in arguments.iter().zip(&foreign.parameters) {
            converted.push(self.c_argument(*value, *parameter)?.into());
        }
        // A function the module already has with another type, as two libraries may declare
        // one name differently, is called through its address with this declaration's type.
        let call = self.builder.build_indirect_call(
            ty,
            callee.as_global_value().as_pointer_value(),
            &converted,
            "",
        )?;
        match foreign.result {
            Some(result) => Ok(Some(self.typy_result(call, result)?)),
            None => Ok(None),
        }
    }

    /// Calls the runtime's `name`, of `ty`, with `arguments`, and returns its result.
    fn runtime_value(
        &mut self,
        name: &str,
        ty: FunctionType<'ctx>,
        arguments: &[BasicMetadataValueEnum<'ctx>],
    ) -> Result<BasicValueEnum<'ctx>, CodegenError> {
        let function = runtime_symbol(self.shared, name, ty);
        self.builder
            .build_call(function, arguments, "")?
            .try_as_basic_value()
            .basic()
            .ok_or_else(|| CodegenError(format!("{name} returns a value")))
    }

    /// Returns a TyPy value as C type `ty` takes it.
    fn c_argument(
        &mut self,
        value: ValueId,
        ty: CType,
    ) -> Result<BasicValueEnum<'ctx>, CodegenError> {
        let context = self.shared.context;
        let (word, pointer) = (
            context.i64_type(),
            context.ptr_type(AddressSpace::default()),
        );
        let lowered = self.value(value)?;
        let held = self.function.value(value).ty;
        // A `ctypes` object, or `None` in its place, passes the address it gives (ADR-0048).
        if is_cdata(held) {
            return self.runtime_value(
                "typy_rt_cdata_argument",
                pointer.fn_type(&[pointer.into()], false),
                &[lowered.into()],
            );
        }
        if let Some((bits, signed)) = ty.integer() {
            let low = self.runtime_value(
                "typy_rt_int_low_bits",
                word.fn_type(&[word.into()], false),
                &[lowered.into()],
            )?;
            if bits == 64 {
                return Ok(low);
            }
            // The value wraps to the type's width, then widens back to the word it passes
            // in, by its sign for a signed type.
            let narrow = match bits {
                8 => context.i8_type(),
                16 => context.i16_type(),
                _ => context.i32_type(),
            };
            let wrapped = self
                .builder
                .build_int_truncate(low.into_int_value(), narrow, "")?;
            let widened = if signed {
                self.builder.build_int_s_extend(wrapped, word, "")?
            } else {
                self.builder.build_int_z_extend(wrapped, word, "")?
            };
            return Ok(widened.into());
        }
        Ok(match ty {
            CType::Float => self
                .builder
                .build_float_trunc(lowered.into_float_value(), context.f32_type(), "")?
                .into(),
            CType::Bool => self
                .builder
                .build_int_z_extend(lowered.into_int_value(), word, "")?
                .into(),
            CType::VoidPointer
                if matches!(held, TyirType::Int | TyirType::Optional(_)) && !is_bytes(held) =>
            {
                let low = self.runtime_value(
                    "typy_rt_int_low_bits",
                    word.fn_type(&[word.into()], false),
                    &[lowered.into()],
                )?;
                self.builder
                    .build_int_to_ptr(low.into_int_value(), pointer, "")?
                    .into()
            }
            CType::CharPointer | CType::VoidPointer => self.runtime_value(
                "typy_rt_bytes_data",
                pointer.fn_type(&[pointer.into()], false),
                &[lowered.into()],
            )?,
            _ => lowered,
        })
    }

    /// Returns the result of a call of C type `ty` as its TyPy value.
    fn typy_result(
        &mut self,
        call: CallSiteValue<'ctx>,
        ty: CType,
    ) -> Result<BasicValueEnum<'ctx>, CodegenError> {
        let context = self.shared.context;
        let (word, pointer) = (
            context.i64_type(),
            context.ptr_type(AddressSpace::default()),
        );
        let result = call
            .try_as_basic_value()
            .basic()
            .ok_or_else(|| CodegenError("a C function of a result returns one".to_owned()))?;
        if let Some((bits, signed)) = ty.integer() {
            if bits < 64 {
                // A value of 32 bits or fewer is an inline `int`: shifted left by one, with
                // the lowest bit set.
                let wide = if signed {
                    self.builder
                        .build_int_s_extend(result.into_int_value(), word, "")?
                } else {
                    self.builder
                        .build_int_z_extend(result.into_int_value(), word, "")?
                };
                let shifted = self
                    .builder
                    .build_left_shift(wide, word.const_int(1, false), "")?;
                return Ok(self
                    .builder
                    .build_or(shifted, word.const_int(1, false), "")?
                    .into());
            }
            let name = if signed {
                "typy_rt_int_from_int64"
            } else {
                "typy_rt_int_from_uint64"
            };
            return self.runtime_value(name, word.fn_type(&[word.into()], false), &[result.into()]);
        }
        Ok(match ty {
            CType::Float => self
                .builder
                .build_float_ext(result.into_float_value(), context.f64_type(), "")?
                .into(),
            CType::Bool => self
                .builder
                .build_int_compare(
                    inkwell::IntPredicate::NE,
                    result.into_int_value(),
                    context.i8_type().const_zero(),
                    "",
                )?
                .into(),
            CType::CharPointer => self.runtime_value(
                "typy_rt_bytes_from_c_string",
                pointer.fn_type(&[pointer.into()], false),
                &[result.into()],
            )?,
            CType::VoidPointer => self.runtime_value(
                "typy_rt_int_from_address",
                word.fn_type(&[pointer.into()], false),
                &[result.into()],
            )?,
            CType::Pointer(class) => {
                let class = crate::cdata::cdata_class(self.shared, class)?;
                self.runtime_value(
                    "typy_rt_cdata_from_address",
                    pointer.fn_type(&[pointer.into(), pointer.into()], false),
                    &[class.into(), result.into()],
                )?
            }
            _ => result,
        })
    }
}

/// Returns whether a value of type `ty` is a `ctypes` object, or an optional one.
fn is_cdata(ty: TyirType) -> bool {
    use typy_tyir::{OptionalBase, TypeId};
    ty == TyirType::CDATA || ty == TyirType::Optional(OptionalBase::Object(TypeId::CDATA))
}

/// Returns whether a value of type `ty` is `bytes`, or an optional `bytes`.
fn is_bytes(ty: TyirType) -> bool {
    use typy_tyir::{OptionalBase, TypeId};
    ty == TyirType::Object(TypeId::BYTES)
        || ty == TyirType::Optional(OptionalBase::Object(TypeId::BYTES))
}
