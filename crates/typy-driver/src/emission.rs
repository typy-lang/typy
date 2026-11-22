//! The output of the stage `--emit` stops at.

/// What `--emit` produced: text for every stage but `obj`, whose object file is bytes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Emission {
    /// Deterministic text (CLI-04).
    Text(String),
    /// An object file.
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
