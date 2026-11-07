//! Symbol names of program functions (detailed design section 10.3).

/// The symbol of the main module's body, which the runtime's `main` calls.
pub(crate) const ENTRY_SYMBOL: &str = "__typy_main";

/// Returns `_TY`, then the length and text of the module name, then those of the function name.
pub(crate) fn symbol(module: &str, function: &str) -> String {
    format!("_TY{}{module}{}{function}", module.len(), function.len())
}

#[cfg(test)]
mod tests {
    use super::symbol;

    #[test]
    fn symbols_carry_the_length_of_each_part() {
        assert_eq!(symbol("__main__", "fib"), "_TY8__main__3fib");
        assert_eq!(symbol("app.models", "load"), "_TY10app.models4load");
    }
}
