//! Checks the rules every TyIR module keeps; it runs after every pass (detailed design section 8.2).

mod definitions;
mod dominance;
mod typing;

use std::fmt;

use crate::block::BlockId;
use crate::module::TyirModule;

/// A broken TyIR rule: an internal compiler error, reported as E9001 by the caller.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerifyError {
    /// The function's symbol.
    pub function: String,
    /// The block, when the error is in one.
    pub block: Option<BlockId>,
    /// What is wrong.
    pub message: String,
}

impl fmt::Display for VerifyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.block {
            Some(BlockId(block)) => {
                write!(formatter, "{} b{block}: {}", self.function, self.message)
            }
            None => write!(formatter, "{}: {}", self.function, self.message),
        }
    }
}

impl std::error::Error for VerifyError {}

/// Checks every function of the module.
pub fn verify(module: &TyirModule) -> Result<(), VerifyError> {
    if module.entry.0 as usize >= module.functions.len() {
        return Err(VerifyError {
            function: "<module>".to_owned(),
            block: None,
            message: "the entry function does not exist".to_owned(),
        });
    }
    for function in &module.functions {
        // Control enters a function at its first block, so a function without one has no body.
        if function.blocks.is_empty() {
            return Err(VerifyError {
                function: function.symbol.clone(),
                block: None,
                message: "the function has no entry block".to_owned(),
            });
        }
        definitions::check(function)?;
        typing::check(module, function)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests;
