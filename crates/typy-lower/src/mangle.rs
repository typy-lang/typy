//! Symbol names of program functions (detailed design section 10.3).

/// The symbol of the main module's body, which the runtime's `main` calls.
pub(crate) const ENTRY_SYMBOL: &str = "__typy_main";

/// The symbol of the function the runtime's `main` calls when the program ends (ADR-0071).
pub(crate) const FINISH_SYMBOL: &str = "__typy_finish";

/// The name of that function, which no function of a program can have.
pub(crate) const FINISH_NAME: &str = "<finish>";

/// Returns `_TY`, then the length and text of the module name, then those of the function name.
pub(crate) fn symbol(module: &str, function: &str) -> String {
    format!("_TY{}{module}{}{function}", module.len(), function.len())
}

#[cfg(test)]
mod tests;
