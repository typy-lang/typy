//! The compiler stages `--emit` can write (CLI-04, CLI-29).

use camino::{Utf8Path, Utf8PathBuf};
use typy_codegen::CodegenOutput;

/// A stage whose output `typy build --emit` writes instead of an executable. The order of the
/// variants is the order of the pipeline.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum EmitStage {
    /// The syntax tree.
    Ast,
    /// The syntax tree with the type of every expression.
    TypedAst,
    /// TyIR before reference counting is shown.
    Tyir,
    /// TyIR with every reference count operation.
    TyirMem,
    /// LLVM IR before the optimisation pipeline runs.
    LlvmIrUnoptimised,
    /// Optimised LLVM IR.
    LlvmIr,
    /// Optimised LLVM bitcode.
    LlvmBc,
    /// Assembly.
    Asm,
    /// An object file.
    Obj,
}

impl EmitStage {
    /// Every stage, in pipeline order.
    pub const ALL: [Self; 9] = [
        Self::Ast,
        Self::TypedAst,
        Self::Tyir,
        Self::TyirMem,
        Self::LlvmIrUnoptimised,
        Self::LlvmIr,
        Self::LlvmBc,
        Self::Asm,
        Self::Obj,
    ];

    /// Returns the stage's name on the command line.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Ast => "ast",
            Self::TypedAst => "typed-ast",
            Self::Tyir => "tyir",
            Self::TyirMem => "tyir-mem",
            Self::LlvmIrUnoptimised => "llvm-ir-unoptimised",
            Self::LlvmIr => "llvm-ir",
            Self::LlvmBc => "llvm-bc",
            Self::Asm => "asm",
            Self::Obj => "obj",
        }
    }

    /// Returns the stage a command line name selects, or `None` for any other text.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|stage| stage.name() == text)
    }

    /// Returns true when the output is bytes rather than text.
    #[must_use]
    pub const fn is_binary(self) -> bool {
        matches!(self, Self::LlvmBc | Self::Obj)
    }

    /// Returns what code generation writes for the stage, or `None` for a stage before it.
    #[must_use]
    pub const fn codegen_output(self) -> Option<CodegenOutput> {
        match self {
            Self::LlvmIrUnoptimised => Some(CodegenOutput::UnoptimisedIr),
            Self::LlvmIr => Some(CodegenOutput::LlvmIr),
            Self::LlvmBc => Some(CodegenOutput::Bitcode),
            Self::Asm => Some(CodegenOutput::Assembly),
            Self::Obj => Some(CodegenOutput::Object),
            Self::Ast | Self::TypedAst | Self::Tyir | Self::TyirMem => None,
        }
    }

    /// Returns the extension of the stage's file, as `rustc --emit` names LLVM's: `ll`, `bc`,
    /// `s` and `o`, and the stage's name for the others.
    #[must_use]
    pub const fn extension(self) -> &'static str {
        match self {
            Self::LlvmIrUnoptimised => "unoptimised.ll",
            Self::LlvmIr => "ll",
            Self::LlvmBc => "bc",
            Self::Asm => "s",
            Self::Obj => "o",
            Self::Ast | Self::TypedAst | Self::Tyir | Self::TyirMem => self.name(),
        }
    }

    /// Returns the path of the stage's file beside `output`: its name up to the first dot,
    /// with the stage's extension, so `app` gives `app.ll` and `libtools.so.1` `libtools.ll`.
    #[must_use]
    pub fn path_beside(self, output: &Utf8Path) -> Utf8PathBuf {
        let name = output.file_name().unwrap_or("output");
        let stem = name
            .split('.')
            .next()
            .filter(|stem| !stem.is_empty())
            .unwrap_or(name);
        output.with_file_name(format!("{stem}.{}", self.extension()))
    }
}

#[cfg(test)]
mod tests;
