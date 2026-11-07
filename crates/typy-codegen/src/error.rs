//! Errors of code generation, which are internal: a verified TyIR module always generates.

use std::fmt;

/// A failure inside code generation or LLVM, reported as E9001.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CodegenError(pub(crate) String);

impl fmt::Display for CodegenError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl From<inkwell::builder::BuilderError> for CodegenError {
    fn from(error: inkwell::builder::BuilderError) -> Self {
        Self(format!("LLVM builder: {error}"))
    }
}

#[cfg(test)]
mod tests {
    use super::CodegenError;

    #[test]
    fn errors_show_their_message() {
        assert_eq!(CodegenError("x".to_owned()).to_string(), "x");
    }
}
