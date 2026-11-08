//! Turns TyIR into LLVM IR and object code (the Code generation context).
#![forbid(unsafe_code)]

mod abi;
mod arithmetic;
mod call;
mod comparison;
mod constant;
mod emit;
mod error;
mod function;
mod layout;
mod object;
mod operation;
mod optimization;
mod runtime;
mod statics;
mod target;
mod terminator;
mod tuple;

use typy_diagnostics::{Diagnostic, DiagnosticCode, Diagnostics, RuleId, SourceLookup};
use typy_tyir::TyirModule;

pub use layout::{
    BIG_INT_TYPE_ID, EXCEPTION_TYPE_ID, IMMORTAL, INLINE_MAX, INLINE_MIN, IntEncoding, STR_TYPE_ID,
    StrKind, encode_int, encode_str,
};
pub use object::ObjectFile;
pub use optimization::Optimization;
pub use target::{Target, TargetTriple};

use emit::{Output, generate};
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

/// Returns the object file of a verified module for `target`; `lookup` gives the file names and lines of traceback entries.
pub fn object(
    module: &TyirModule,
    target: Target,
    optimization: Optimization,
    lookup: &dyn SourceLookup,
) -> Result<ObjectFile, Diagnostics> {
    let bytes = generate(module, target, optimization, lookup, Output::Object)
        .map_err(|error| internal(&error))?;
    Ok(ObjectFile { target, bytes })
}

/// Returns the optimised LLVM IR as text, for `--emit=llvm-ir`.
pub fn llvm_ir(
    module: &TyirModule,
    target: Target,
    optimization: Optimization,
    lookup: &dyn SourceLookup,
) -> Result<String, Diagnostics> {
    let bytes = generate(module, target, optimization, lookup, Output::LlvmIr)
        .map_err(|error| internal(&error))?;
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

/// Returns the assembly text, for `--emit=asm`.
pub fn assembly(
    module: &TyirModule,
    target: Target,
    optimization: Optimization,
    lookup: &dyn SourceLookup,
) -> Result<String, Diagnostics> {
    let bytes = generate(module, target, optimization, lookup, Output::Assembly)
        .map_err(|error| internal(&error))?;
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

/// Returns the version of the LLVM library TyPy is linked with, as `(major, minor, patch)`.
#[must_use]
pub fn llvm_version() -> (u32, u32, u32) {
    inkwell::support::get_llvm_version()
}

#[cfg(test)]
mod tests {
    use super::llvm_version;

    #[test]
    fn the_linked_llvm_is_major_version_23() {
        assert_eq!(llvm_version().0, 23);
    }
}
