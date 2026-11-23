//! What code generation writes for a module (CLI-04, CLI-29).

/// What code generation writes for a module.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CodegenOutput {
    /// LLVM IR as text, before the optimisation pipeline runs.
    UnoptimisedIr,
    /// LLVM IR as text, after it.
    LlvmIr,
    /// LLVM bitcode, after it.
    Bitcode,
    /// Assembly text.
    Assembly,
    /// An object file.
    Object,
}

impl CodegenOutput {
    /// Returns true for assembly and object code, which LLVM's code generator writes.
    pub(crate) const fn is_machine_code(self) -> bool {
        matches!(self, Self::Assembly | Self::Object)
    }
}
