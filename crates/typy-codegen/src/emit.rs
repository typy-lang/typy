//! Builds the LLVM module for a TyIR module, runs LLVM's pipeline, and writes object code, assembly or IR.

use inkwell::attributes::{Attribute, AttributeLoc};
use inkwell::context::Context;
use inkwell::module::{Linkage, Module};
use inkwell::passes::PassBuilderOptions;
use inkwell::targets::{
    CodeModel, FileType, InitializationConfig, RelocMode, Target as LlvmTarget, TargetMachine,
    TargetTriple as LlvmTriple,
};
use inkwell::values::FunctionValue;
use typy_diagnostics::SourceLookup;
use typy_tyir::{TyirModule, TyirType};

use crate::abi::function_type;
use crate::error::CodegenError;
use crate::function::{FunctionEmitter, Shared};
use crate::optimization::Optimization;
use crate::statics::Statics;
use crate::target::Target;

/// What to write: object code, assembly text or LLVM IR text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Output {
    Object,
    Assembly,
    LlvmIr,
}

fn target_machine(
    target: Target,
    optimization: Optimization,
) -> Result<TargetMachine, CodegenError> {
    LlvmTarget::initialize_x86(&InitializationConfig::default());
    let triple = LlvmTriple::create(target.triple.llvm_triple());
    let llvm_target = LlvmTarget::from_triple(&triple)
        .map_err(|error| CodegenError(format!("target {}: {error}", target.triple)))?;
    // The baseline CPU of the triple, so every machine of the target runs the output and the bytes do not depend on the build host.
    llvm_target
        .create_target_machine(
            &triple,
            "x86-64",
            "",
            optimization.level(),
            RelocMode::PIC,
            CodeModel::Default,
        )
        .ok_or_else(|| CodegenError(format!("LLVM cannot generate code for {}", target.triple)))
}

/// Returns the bytes of the requested output for `module`.
pub(crate) fn generate(
    module: &TyirModule,
    target: Target,
    optimization: Optimization,
    lookup: &dyn SourceLookup,
    output: Output,
) -> Result<Vec<u8>, CodegenError> {
    let machine = target_machine(target, optimization)?;
    let context = Context::create();
    let llvm_module = context.create_module("typy");
    llvm_module.set_triple(&machine.get_triple());
    llvm_module.set_data_layout(&machine.get_target_data().get_data_layout());
    let raising: Vec<bool> = module
        .functions
        .iter()
        .map(|function| function.may_raise || function.id == module.entry)
        .collect();
    let functions = declare_functions(&context, &llvm_module, module, &raising);
    if optimization.minimise_size() {
        mark_minimum_size(&context, &functions);
    }
    let mut statics = Statics::new(&context);
    let mut shared = Shared {
        context: &context,
        module: &llvm_module,
        statics: &mut statics,
        tyir: module,
        functions: &functions,
        raising: &raising,
        lookup,
    };
    for function in &module.functions {
        FunctionEmitter::emit(&mut shared, function)?;
    }
    llvm_module
        .verify()
        .map_err(|error| CodegenError(format!("LLVM rejected the module: {error}")))?;
    llvm_module
        .run_passes(
            optimization.pipeline(),
            &machine,
            PassBuilderOptions::create(),
        )
        .map_err(|error| CodegenError(format!("LLVM passes: {error}")))?;
    match output {
        Output::LlvmIr => Ok(llvm_module.print_to_string().to_bytes().to_vec()),
        Output::Object | Output::Assembly => {
            let file_type = if output == Output::Object {
                FileType::Object
            } else {
                FileType::Assembly
            };
            let buffer = machine
                .write_to_memory_buffer(&llvm_module, file_type)
                .map_err(|error| CodegenError(format!("LLVM code generation: {error}")))?;
            Ok(buffer.as_slice().to_vec())
        }
    }
}

/// Marks each function to be optimised for size, as `--release-small` asks.
fn mark_minimum_size<'ctx>(context: &'ctx Context, functions: &[FunctionValue<'ctx>]) {
    for name in ["minsize", "optsize"] {
        let attribute = context.create_enum_attribute(Attribute::get_named_enum_kind_id(name), 0);
        for function in functions {
            function.add_attribute(AttributeLoc::Function, attribute);
        }
    }
}

/// Declares every function first, so calls can refer to functions defined later; only the entry is visible outside the object.
fn declare_functions<'ctx>(
    context: &'ctx Context,
    llvm_module: &Module<'ctx>,
    module: &TyirModule,
    raising: &[bool],
) -> Vec<FunctionValue<'ctx>> {
    module
        .functions
        .iter()
        .zip(raising)
        .map(|(function, raising)| {
            let parameters: Vec<TyirType> = function
                .parameters
                .iter()
                .map(|value| function.value(*value).ty)
                .collect();
            let ty = function_type(
                context,
                &module.tuple_types,
                &parameters,
                function.return_type,
                *raising,
                false,
            );
            let linkage = if function.id == module.entry {
                Linkage::External
            } else {
                Linkage::Internal
            };
            llvm_module.add_function(&function.symbol, ty, Some(linkage))
        })
        .collect()
}
