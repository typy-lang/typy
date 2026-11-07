//! The whole program after checking.

use super::function::{FunctionId, TypedFunction};
use super::symbol::{Symbol, SymbolId};

/// The whole program after checking.
#[derive(Clone, Debug, PartialEq)]
pub struct TypedProgram {
    /// Every symbol, indexed by `SymbolId`.
    pub symbols: Vec<Symbol>,
    /// Every function, indexed by `FunctionId`; the module body is one of them.
    pub functions: Vec<TypedFunction>,
    /// The module body of `__main__`.
    pub entry: FunctionId,
}

impl TypedProgram {
    /// Returns a symbol by its ID.
    #[must_use]
    pub fn symbol(&self, id: SymbolId) -> &Symbol {
        &self.symbols[id.0 as usize]
    }

    /// Returns a function by its ID.
    #[must_use]
    pub fn function(&self, id: FunctionId) -> &TypedFunction {
        &self.functions[id.0 as usize]
    }
}
