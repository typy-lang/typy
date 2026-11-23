//! The output of one stage `--emit` names.

/// What `--emit` produced for one stage: text, or bytes for `llvm-bc` and `obj`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Emission {
    /// Deterministic text (CLI-04).
    Text(String),
    /// LLVM bitcode or an object file.
    Bytes(Vec<u8>),
}

impl Emission {
    /// Returns the output as bytes, to write to a file or to standard output.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        match self {
            Self::Text(text) => text.as_bytes(),
            Self::Bytes(bytes) => bytes,
        }
    }
}

#[cfg(test)]
mod tests;
