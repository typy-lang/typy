//! The whole program after checking.

use super::class::{ClassId, TypedClass};
use super::function::{FunctionId, TypedFunction};
use super::symbol::{Symbol, SymbolId};

/// The whole program after checking.
#[derive(Clone, Debug, PartialEq)]
pub struct TypedProgram {
    /// Every symbol, indexed by `SymbolId`.
    pub symbols: Vec<Symbol>,
    /// Every function, indexed by `FunctionId`; the module body is one of them.
    pub functions: Vec<TypedFunction>,
    /// Every class, indexed by `ClassId`.
    pub classes: Vec<TypedClass>,
    /// Every module, each after the modules its module-level imports need (ADR-0015).
    pub modules: Vec<TypedModule>,
    /// The module body of `__main__`.
    pub entry: FunctionId,
}

/// A module of the program after checking.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TypedModule {
    /// The module's qualified name, `__main__` for the main file.
    pub name: String,
    /// The path of its file from the project root, which its text shows (OI-20).
    pub path: String,
    /// The function of its module-level code.
    pub body: FunctionId,
}

impl TypedProgram {
    /// Returns a symbol by its ID.
    #[must_use]
    pub fn symbol(&self, id: SymbolId) -> &Symbol {
        &self.symbols[id.0 as usize]
    }

    /// Returns a class by its ID.
    #[must_use]
    pub fn class(&self, id: ClassId) -> &TypedClass {
        &self.classes[id.0 as usize]
    }

    /// Returns a function by its ID.
    #[must_use]
    pub fn function(&self, id: FunctionId) -> &TypedFunction {
        &self.functions[id.0 as usize]
    }
}
