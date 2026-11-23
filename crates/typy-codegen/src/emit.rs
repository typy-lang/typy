//! Builds the LLVM module for a TyIR module, runs LLVM's pipeline, and writes object code,
//! assembly, IR or bitcode.

use inkwell::attributes::{Attribute, AttributeLoc};
use inkwell::context::Context;
use inkwell::module::{Linkage, Module};
use inkwell::passes::PassBuilderOptions;
use inkwell::targets::{
    CodeModel, FileType, InitializationConfig, RelocMode, Target as LlvmTarget, TargetMachine,
    TargetTriple as LlvmTriple,
};
use inkwell::values::{FunctionValue, PointerValue};
use typy_diagnostics::SourceLookup;
use typy_tyir::{TyirModule, TyirType};

use crate::abi::function_type;
use crate::error::CodegenError;
use crate::function::{FunctionEmitter, Shared};
use crate::heap::emit_heap_hooks;
use crate::library::LibraryEmitter;
use crate::optimization::Optimization;
use crate::options::CodegenOptions;
use crate::output::CodegenOutput;
use crate::statics::Statics;
use crate::target::Target;

/// Returns the processor of the machine `typy` runs on, as LLVM names it, such as `skylake`.
pub(crate) fn host_processor() -> String {
    TargetMachine::get_host_cpu_name().to_string()
}

/// Returns the features of the machine `typy` runs on, as LLVM writes them.
pub(crate) fn host_features() -> String {
    TargetMachine::get_host_cpu_features().to_string()
}

/// Writes LLVM's list of the processors and features of `triple` to standard error, as
/// `llc -mcpu=help` does; LLVM writes it when a machine is made for the processor `help`.
pub(crate) fn write_processor_help(
    triple: crate::target::TargetTriple,
) -> Result<(), CodegenError> {
    LlvmTarget::initialize_x86(&InitializationConfig::default());
    LlvmTarget::initialize_aarch64(&InitializationConfig::default());
    let llvm_triple = LlvmTriple::create(triple.llvm_triple());
    let llvm_target = LlvmTarget::from_triple(&llvm_triple)
        .map_err(|error| CodegenError(format!("target {triple}: {error}")))?;
    llvm_target
        .create_target_machine(
            &llvm_triple,
            "help",
            "",
            inkwell::OptimizationLevel::None,
            RelocMode::PIC,
            CodeModel::Default,
        )
        .map(|_| ())
        .ok_or_else(|| CodegenError(format!("LLVM cannot generate code for {triple}")))
}

fn target_machine(target: Target, options: &CodegenOptions) -> Result<TargetMachine, CodegenError> {
    LlvmTarget::initialize_x86(&InitializationConfig::default());
    LlvmTarget::initialize_aarch64(&InitializationConfig::default());
    let triple = LlvmTriple::create(target.triple.llvm_triple());
    let llvm_target = LlvmTarget::from_triple(&triple)
        .map_err(|error| CodegenError(format!("target {}: {error}", target.triple)))?;
    llvm_target
        .create_target_machine(
            &triple,
            options.processor.as_deref().unwrap_or(target.triple.cpu()),
            &options.features,
            options.optimization.level(),
            RelocMode::PIC,
            CodeModel::Default,
        )
        .ok_or_else(|| CodegenError(format!("LLVM cannot generate code for {}", target.triple)))
}

/// Declares each global of the module as an internal LLVM variable of zeros, which the entry
/// function fills before any module code runs; a global of a type with no value has none.
fn declare_globals<'ctx>(
    context: &'ctx Context,
    llvm_module: &Module<'ctx>,
    module: &TyirModule,
) -> Vec<Option<PointerValue<'ctx>>> {
    module
        .globals
        .iter()
        .enumerate()
        .map(|(index, global)| {
            let ty = crate::abi::value_type(context, &module.tuple_types, global.ty)?;
            let variable = llvm_module.add_global(ty, None, &format!("typy.global.{index}"));
            variable.set_initializer(&ty.const_zero());
            variable.set_linkage(Linkage::Internal);
            Some(variable.as_pointer_value())
        })
        .collect()
}

/// Returns the bytes of each of `outputs` for `module`, in their order, from one LLVM module.
pub(crate) fn generate(
    module: &TyirModule,
    target: Target,
    options: &CodegenOptions,
    lookup: &dyn SourceLookup,
    outputs: &[CodegenOutput],
) -> Result<Vec<Vec<u8>>, CodegenError> {
    let optimization = options.optimization;
    let machine = target_machine(target, options)?;
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
    mark_size(&context, &functions, optimization);
    let globals = declare_globals(&context, &llvm_module, module);
    let mut statics = Statics::new(&context);
    let mut shared = Shared {
        context: &context,
        module: &llvm_module,
        statics: &mut statics,
        tyir: module,
        functions: &functions,
        globals: &globals,
        raising: &raising,
        lookup,
        target: target.triple,
        closes_generators: module
            .functions
            .iter()
            .any(|function| function.generator.is_some()),
    };
    for function in &module.functions {
        FunctionEmitter::emit(&mut shared, function)?;
    }
    if let Some(library) = &module.library {
        LibraryEmitter {
            context: &context,
            module: &llvm_module,
            tyir: module,
            functions: &functions,
            raising: &raising,
            target: target.triple,
        }
        .emit(library)?;
    }
    emit_heap_hooks(&context, &llvm_module, &module.allocator)?;
    llvm_module
        .verify()
        .map_err(|error| CodegenError(format!("LLVM rejected the module: {error}")))?;
    let text = |module: &Module<'_>| module.print_to_string().to_bytes().to_vec();
    let mut written = vec![Vec::new(); outputs.len()];
    for (index, output) in outputs.iter().enumerate() {
        if *output == CodegenOutput::UnoptimisedIr {
            written[index] = text(&llvm_module);
        }
    }
    if outputs
        .iter()
        .all(|output| *output == CodegenOutput::UnoptimisedIr)
    {
        return Ok(written);
    }
    llvm_module
        .run_passes(
            optimization.pipeline(),
            &machine,
            PassBuilderOptions::create(),
        )
        .map_err(|error| CodegenError(format!("LLVM passes: {error}")))?;
    let last_machine_code = outputs.iter().rposition(|output| output.is_machine_code());
    for (index, output) in outputs.iter().enumerate() {
        written[index] = match output {
            CodegenOutput::UnoptimisedIr => continue,
            CodegenOutput::LlvmIr => text(&llvm_module),
            CodegenOutput::Bitcode => llvm_module.write_bitcode_to_memory().as_slice().to_vec(),
            CodegenOutput::Assembly | CodegenOutput::Object => {
                let file_type = if *output == CodegenOutput::Object {
                    FileType::Object
                } else {
                    FileType::Assembly
                };
                // LLVM's code generator changes the module it writes, so every output but the
                // last is written from a copy.
                let copy;
                let source = if Some(index) == last_machine_code {
                    &llvm_module
                } else {
                    copy = llvm_module.clone();
                    &copy
                };
                let buffer = machine
                    .write_to_memory_buffer(source, file_type)
                    .map_err(|error| CodegenError(format!("LLVM code generation: {error}")))?;
                buffer.as_slice().to_vec()
            }
        };
    }
    Ok(written)
}

/// Marks each function `optsize` for `-Os`, and `minsize` too for `-Oz`.
fn mark_size<'ctx>(
    context: &'ctx Context,
    functions: &[FunctionValue<'ctx>],
    optimization: Optimization,
) {
    let names: &[&str] = match (optimization.minimise_size(), optimization.optimise_size()) {
        (true, _) => &["minsize", "optsize"],
        (false, true) => &["optsize"],
        (false, false) => &[],
    };
    for name in names {
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
            let linkage = if function.id == module.entry || function.id == module.finish {
                Linkage::External
            } else {
                Linkage::Internal
            };
            llvm_module.add_function(&function.symbol, ty, Some(linkage))
        })
        .collect()
}
