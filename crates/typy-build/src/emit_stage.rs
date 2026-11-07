//! The compiler stages `--emit` can write (CLI-04).

/// A stage whose output `typy build --emit` writes instead of an executable.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum EmitStage {
    /// The syntax tree.
    Ast,
    /// The syntax tree with the type of every expression.
    TypedAst,
    /// TyIR before reference counting is shown.
    Tyir,
    /// TyIR with every reference count operation.
    TyirMem,
    /// Optimised LLVM IR.
    LlvmIr,
    /// Assembly.
    Asm,
    /// An object file.
    Obj,
}

impl EmitStage {
    /// Every stage, in pipeline order.
    pub const ALL: [Self; 7] = [
        Self::Ast,
        Self::TypedAst,
        Self::Tyir,
        Self::TyirMem,
        Self::LlvmIr,
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
            Self::LlvmIr => "llvm-ir",
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
        matches!(self, Self::Obj)
    }
}

#[cfg(test)]
mod tests {
    use super::EmitStage;

    #[test]
    fn every_stage_parses_from_its_name() {
        for stage in EmitStage::ALL {
            assert_eq!(EmitStage::parse(stage.name()), Some(stage));
            assert_eq!(stage.is_binary(), stage == EmitStage::Obj);
        }
    }

    #[test]
    fn other_names_are_refused() {
        for text in ["", "AST", "llvm", "typed_ast", "exe"] {
            assert_eq!(EmitStage::parse(text), None, "{text:?}");
        }
    }
}
