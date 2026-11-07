//! The whole program in TyIR.

use crate::constant::StringId;
use crate::function::{FunctionId, TyirFunction};

/// Every function of the program, the string pool, and the entry function.
#[derive(Clone, Debug, PartialEq)]
pub struct TyirModule {
    /// The functions, indexed by `FunctionId`.
    pub functions: Vec<TyirFunction>,
    /// The string constants, indexed by `StringId`.
    pub strings: Vec<String>,
    /// The module body of `__main__`.
    pub entry: FunctionId,
}

impl TyirModule {
    /// Returns a function by its ID.
    #[must_use]
    pub fn function(&self, id: FunctionId) -> &TyirFunction {
        &self.functions[id.0 as usize]
    }

    /// Returns a string constant by its ID.
    #[must_use]
    pub fn string(&self, id: StringId) -> &str {
        &self.strings[id.0 as usize]
    }
}
