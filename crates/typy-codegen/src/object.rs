//! The machine code of one program for one target.

use crate::target::Target;

/// An object file in memory.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObjectFile {
    /// The target it was generated for.
    pub target: Target,
    /// The bytes of the object file.
    pub bytes: Vec<u8>,
}
