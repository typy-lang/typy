//! Emits what the object of a library holds besides the program: a constructor that runs the
//! module bodies when the library is loaded, a destructor that ends it as a program ends, and
//! a wrapper with C types for each export (CLI-09, FFI-05, EXC-05, ADR-0044).

use inkwell::AddressSpace;
use inkwell::attributes::{Attribute, AttributeLoc};
use inkwell::builder::Builder;
use inkwell::context::Context;
use inkwell::module::{Linkage, Module};
use inkwell::types::{BasicMetadataTypeEnum, BasicType, BasicTypeEnum, FunctionType};
use inkwell::values::{BasicMetadataValueEnum, BasicValueEnum, FunctionValue, PointerValue};
use typy_tyir::{CType, TyirExport, TyirLibrary, TyirModule};

use crate::error::CodegenError;

/// The priority of the constructor and the destructor: the lowest, so they run after those
/// of the C library and the host.
const PRIORITY: u64 = 65_535;

/// The runtime's functions a library's object calls.
const LOAD: &str = "typy_rt_library_load";
const UNLOAD: &str = "typy_rt_library_unload";
const ENTER: &str = "typy_rt_library_enter";
const LEAVE: &str = "typy_rt_library_leave";
const RAISED: &str = "typy_rt_library_raised";
const FROM_INT64: &str = "typy_rt_int_from_int64";
const INTO_INT64: &str = "typy_rt_int_into_int64";
const INT_DECREMENT: &str = "typy_rt_int_decrement";

/// What emitting a library reads: the LLVM module and the program's functions.
pub(crate) struct LibraryEmitter<'module, 'ctx> {
    pub(crate) context: &'ctx Context,
    pub(crate) module: &'module Module<'ctx>,
    pub(crate) tyir: &'module TyirModule,
    pub(crate) functions: &'module [FunctionValue<'ctx>],
    pub(crate) raising: &'module [bool],
}

impl<'ctx> LibraryEmitter<'_, 'ctx> {
    /// Emits the constructor, the destructor and the exports of `library`.
    pub(crate) fn emit(&self, library: &TyirLibrary) -> Result<(), CodegenError> {
        let pointer = self.context.ptr_type(AddressSpace::default());
        let void = self.context.void_type();
        let entry = self.functions[self.tyir.entry.0 as usize];
        let load = self.declare(LOAD, void.fn_type(&[pointer.into()], false));
        let unload = self.declare(UNLOAD, void.fn_type(&[], false));
        let constructor = self.call_once(
            "typy.library.load",
            load,
            &[entry.as_global_value().as_pointer_value().into()],
        )?;
        let destructor = self.call_once("typy.library.unload", unload, &[])?;
        self.list("llvm.global_ctors", constructor);
        self.list("llvm.global_dtors", destructor);
        for export in &library.exports {
            self.export(export)?;
        }
        Ok(())
    }

    fn declare(&self, name: &str, ty: FunctionType<'ctx>) -> FunctionValue<'ctx> {
        self.module
            .get_function(name)
            .unwrap_or_else(|| self.module.add_function(name, ty, Some(Linkage::External)))
    }

    /// Defines an internal function that makes one call of `callee` with `arguments`.
    fn call_once(
        &self,
        name: &str,
        callee: FunctionValue<'ctx>,
        arguments: &[BasicMetadataValueEnum<'ctx>],
    ) -> Result<FunctionValue<'ctx>, CodegenError> {
        let function = self.module.add_function(
            name,
            self.context.void_type().fn_type(&[], false),
            Some(Linkage::Internal),
        );
        let builder = self.context.create_builder();
        builder.position_at_end(self.context.append_basic_block(function, ""));
        builder.build_call(callee, arguments, "")?;
        builder.build_return(None)?;
        Ok(function)
    }

    /// Adds `function` to the list of constructors or destructors `name` names.
    fn list(&self, name: &str, function: FunctionValue<'ctx>) {
        let pointer = self.context.ptr_type(AddressSpace::default());
        let priority = self.context.i32_type().const_int(PRIORITY, false);
        let entry_type = self.context.struct_type(
            &[
                self.context.i32_type().into(),
                pointer.into(),
                pointer.into(),
            ],
            false,
        );
        let entry = entry_type.const_named_struct(&[
            priority.into(),
            function.as_global_value().as_pointer_value().into(),
            pointer.const_null().into(),
        ]);
        let entries = entry_type.const_array(&[entry]);
        let global = self.module.add_global(entries.get_type(), None, name);
        global.set_linkage(Linkage::Appending);
        global.set_initializer(&entries);
    }

    fn c_type(&self, ty: CType) -> BasicTypeEnum<'ctx> {
        match ty {
            CType::Int64 => self.context.i64_type().into(),
            CType::Double => self.context.f64_type().into(),
            // C passes and returns a `bool` as one byte holding 0 or 1.
            CType::Bool => self.context.i8_type().into(),
        }
    }

    /// Defines the export: a function with C types that converts each argument, calls the
    /// program's function, and converts its result.
    fn export(&self, export: &TyirExport) -> Result<(), CodegenError> {
        if self.module.get_function(&export.name).is_some() {
            return Err(CodegenError(format!(
                "the export `{}` has the name of another function of the object",
                export.name
            )));
        }
        let parameters: Vec<BasicMetadataTypeEnum<'ctx>> = export
            .parameters
            .iter()
            .map(|ty| self.c_type(*ty).into())
            .collect();
        let ty = match export.result {
            Some(result) => self.c_type(result).fn_type(&parameters, false),
            None => self.context.void_type().fn_type(&parameters, false),
        };
        let wrapper = self
            .module
            .add_function(&export.name, ty, Some(Linkage::External));
        let zero_extended = self
            .context
            .create_enum_attribute(Attribute::get_named_enum_kind_id("zeroext"), 0);
        for (index, parameter) in export.parameters.iter().enumerate() {
            if *parameter == CType::Bool {
                let index = u32::try_from(index).unwrap_or(u32::MAX);
                wrapper.add_attribute(AttributeLoc::Param(index), zero_extended);
            }
        }
        if export.result == Some(CType::Bool) {
            wrapper.add_attribute(AttributeLoc::Return, zero_extended);
        }
        let builder = self.context.create_builder();
        builder.position_at_end(self.context.append_basic_block(wrapper, ""));
        let call = ExportCall {
            emitter: self,
            builder: &builder,
            wrapper,
        };
        call.body(export)
    }
}

/// The body of one export's wrapper, built in order.
struct ExportCall<'emitter, 'module, 'ctx> {
    emitter: &'emitter LibraryEmitter<'module, 'ctx>,
    builder: &'emitter Builder<'ctx>,
    wrapper: FunctionValue<'ctx>,
}

impl<'ctx> ExportCall<'_, '_, 'ctx> {
    fn runtime(&self, name: &str, ty: FunctionType<'ctx>) -> FunctionValue<'ctx> {
        self.emitter.declare(name, ty)
    }

    fn body(&self, export: &TyirExport) -> Result<(), CodegenError> {
        let context = self.emitter.context;
        let (word, void) = (context.i64_type(), context.void_type());
        let pointer = context.ptr_type(AddressSpace::default());
        self.builder
            .build_call(self.runtime(ENTER, void.fn_type(&[], false)), &[], "")?;
        let from_int64 = self.runtime(FROM_INT64, word.fn_type(&[word.into()], false));
        let mut arguments: Vec<BasicMetadataValueEnum<'ctx>> = Vec::new();
        let mut owned = Vec::new();
        for (index, ty) in export.parameters.iter().enumerate() {
            let index = u32::try_from(index).unwrap_or(u32::MAX);
            let parameter = self
                .wrapper
                .get_nth_param(index)
                .ok_or_else(|| CodegenError(format!("{} has no parameter {index}", export.name)))?;
            let argument: BasicValueEnum<'ctx> = match ty {
                CType::Int64 => {
                    let converted = call_value(self.builder.build_call(
                        from_int64,
                        &[parameter.into()],
                        "",
                    )?)?;
                    owned.push(converted);
                    converted
                }
                CType::Double => parameter,
                CType::Bool => self
                    .builder
                    .build_int_compare(
                        inkwell::IntPredicate::NE,
                        parameter.into_int_value(),
                        context.i8_type().const_zero(),
                        "",
                    )?
                    .into(),
            };
            arguments.push(argument.into());
        }
        let callee = self.emitter.functions[export.function.0 as usize];
        let returned = self
            .builder
            .build_call(callee, &arguments, "")?
            .try_as_basic_value()
            .basic();
        // The function borrows its arguments; the words the conversion made are the
        // wrapper's to drop (MEM-04).
        let decrement = self.runtime(INT_DECREMENT, void.fn_type(&[word.into()], false));
        for converted in owned {
            self.builder
                .build_call(decrement, &[converted.into()], "")?;
        }
        let raising = self.emitter.raising[export.function.0 as usize];
        let result = match (raising, returned) {
            (true, Some(BasicValueEnum::StructValue(pair))) => {
                let exception = self.builder.build_extract_value(pair, 1, "")?;
                self.leave_on_exception(exception.into_pointer_value())?;
                Some(self.builder.build_extract_value(pair, 0, "")?)
            }
            (true, Some(exception)) => {
                self.leave_on_exception(exception.into_pointer_value())?;
                None
            }
            (_, returned) => returned,
        };
        let converted = match (export.result, result) {
            (Some(CType::Int64), Some(value)) => {
                let pair_type = context.struct_type(&[word.into(), pointer.into()], false);
                let into_int64 = self.runtime(INTO_INT64, pair_type.fn_type(&[word.into()], false));
                let pair = call_value(self.builder.build_call(into_int64, &[value.into()], "")?)?
                    .into_struct_value();
                let exception = self.builder.build_extract_value(pair, 1, "")?;
                self.leave_on_exception(exception.into_pointer_value())?;
                Some(self.builder.build_extract_value(pair, 0, "")?)
            }
            (Some(CType::Bool), Some(value)) => Some(
                self.builder
                    .build_int_z_extend(value.into_int_value(), context.i8_type(), "")?
                    .into(),
            ),
            (Some(CType::Double), Some(value)) => Some(value),
            (None, _) => None,
            (Some(_), None) => {
                return Err(CodegenError(format!(
                    "the function of the export `{}` returns nothing",
                    export.name
                )));
            }
        };
        self.builder
            .build_call(self.runtime(LEAVE, void.fn_type(&[], false)), &[], "")?;
        match converted {
            Some(value) => self.builder.build_return(Some(&value))?,
            None => self.builder.build_return(None)?,
        };
        Ok(())
    }

    /// Ends the process when `exception` is not null, and goes on in a new block when it is.
    fn leave_on_exception(&self, exception: PointerValue<'ctx>) -> Result<(), CodegenError> {
        let context = self.emitter.context;
        let pointer = context.ptr_type(AddressSpace::default());
        let raised = self.runtime(
            RAISED,
            context.void_type().fn_type(&[pointer.into()], false),
        );
        let no_return =
            context.create_enum_attribute(Attribute::get_named_enum_kind_id("noreturn"), 0);
        raised.add_attribute(AttributeLoc::Function, no_return);
        let failed = context.append_basic_block(self.wrapper, "");
        let succeeded = context.append_basic_block(self.wrapper, "");
        let is_null = self.builder.build_is_null(exception, "")?;
        self.builder
            .build_conditional_branch(is_null, succeeded, failed)?;
        self.builder.position_at_end(failed);
        self.builder.build_call(raised, &[exception.into()], "")?;
        self.builder.build_unreachable()?;
        self.builder.position_at_end(succeeded);
        Ok(())
    }
}

/// Returns the result of a call of a runtime conversion, which always has one.
fn call_value(
    call: inkwell::values::CallSiteValue<'_>,
) -> Result<BasicValueEnum<'_>, CodegenError> {
    call.try_as_basic_value()
        .basic()
        .ok_or_else(|| CodegenError("a runtime conversion returns a value".to_owned()))
}
