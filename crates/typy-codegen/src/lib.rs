//! Turns TyIR into LLVM IR and object code (the Code generation context).
#![forbid(unsafe_code)]

mod abi;
mod arithmetic;
mod binding;
mod bits;
mod call;
mod cdata;
mod cell;
mod class;
mod comparison;
mod constant;
mod descriptor;
mod emit;
mod error;
mod foreign;
mod frame;
mod function;
mod function_object;
mod generator;
mod header;
mod heap;
mod layout;
mod library;
mod object;
mod operation;
mod optimization;
mod optional;
mod options;
mod output;
mod runtime;
mod small_int;
mod statics;
mod structure_abi;
mod target;
mod terminator;
mod tuple;

use typy_diagnostics::{Diagnostic, DiagnosticCode, Diagnostics, RuleId, SourceLookup};
use typy_tyir::TyirModule;

pub use binding::{python_binding, python_stub};
pub use header::c_header;
pub use layout::{
    BIG_INT_TYPE_ID, BYTES_TYPE_ID, EXCEPTION_TYPE_ID, IMMORTAL, INLINE_MAX, INLINE_MIN,
    IntEncoding, STR_TYPE_ID, StrKind, encode_int, encode_str,
};
pub use object::ObjectFile;
pub use optimization::Optimization;
pub use options::CodegenOptions;
pub use output::CodegenOutput;
pub use target::{Target, TargetTriple};

use emit::generate;
use error::CodegenError;

fn internal(error: &CodegenError) -> Diagnostics {
    Diagnostics::from(
        Diagnostic::without_location(
            DiagnosticCode::error(9001),
            format!("internal compiler error in code generation: {error}"),
        )
        .with_rule(RuleId::NFR_09)
        .with_help("this is a defect in TyPy; please report it with the program that caused it"),
    )
}

/// Returns the bytes of each of `wanted` for a verified module for `target`, in their order,
/// building the LLVM module once (CLI-29); `lookup` gives the file names and lines of
/// traceback entries.
pub fn outputs(
    module: &TyirModule,
    target: Target,
    options: &CodegenOptions,
    lookup: &dyn SourceLookup,
    wanted: &[CodegenOutput],
) -> Result<Vec<Vec<u8>>, Diagnostics> {
    generate(module, target, options, lookup, wanted).map_err(|error| internal(&error))
}

/// Returns the bytes of one output.
fn one(
    module: &TyirModule,
    target: Target,
    options: &CodegenOptions,
    lookup: &dyn SourceLookup,
    output: CodegenOutput,
) -> Result<Vec<u8>, Diagnostics> {
    let mut written = outputs(module, target, options, lookup, &[output])?;
    Ok(written.pop().unwrap_or_default())
}

/// Returns the object file of a verified module for `target`; `lookup` gives the file names and lines of traceback entries.
pub fn object(
    module: &TyirModule,
    target: Target,
    options: &CodegenOptions,
    lookup: &dyn SourceLookup,
) -> Result<ObjectFile, Diagnostics> {
    let bytes = one(module, target, options, lookup, CodegenOutput::Object)?;
    Ok(ObjectFile { target, bytes })
}

/// Returns the optimised LLVM IR as text, for `--emit=llvm-ir`.
pub fn llvm_ir(
    module: &TyirModule,
    target: Target,
    options: &CodegenOptions,
    lookup: &dyn SourceLookup,
) -> Result<String, Diagnostics> {
    let bytes = one(module, target, options, lookup, CodegenOutput::LlvmIr)?;
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

/// Returns the assembly text, for `--emit=asm`.
pub fn assembly(
    module: &TyirModule,
    target: Target,
    options: &CodegenOptions,
    lookup: &dyn SourceLookup,
) -> Result<String, Diagnostics> {
    let bytes = one(module, target, options, lookup, CodegenOutput::Assembly)?;
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

/// Returns the processor of the machine `typy` runs on, as LLVM names it, such as `skylake`,
/// which `--target-cpu native` builds for (CLI-21).
#[must_use]
pub fn host_processor() -> String {
    emit::host_processor()
}

/// Returns the features of the machine `typy` runs on, as LLVM writes them, such as
/// `+sse2,+avx2,-avx512f`, which `--target-cpu native` builds with (CLI-21).
#[must_use]
pub fn host_features() -> String {
    emit::host_features()
}

/// Writes LLVM's list of the processors and features of `triple` to standard error, as
/// `llc -mcpu=help` does, for `typy print target-cpus` (CLI-25).
///
/// # Errors
///
/// An internal error when LLVM cannot make a machine for the triple.
pub fn write_processor_help(triple: TargetTriple) -> Result<(), Diagnostics> {
    emit::write_processor_help(triple).map_err(|error| internal(&error))
}

/// Returns the version of the LLVM library TyPy is linked with, as `(major, minor, patch)`.
#[must_use]
pub fn llvm_version() -> (u32, u32, u32) {
    inkwell::support::get_llvm_version()
}

#[cfg(test)]
mod tests;
