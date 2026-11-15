//! Literal values after checking.

use crate::types::Type;

/// A literal value after checking.
#[derive(Clone, Debug, PartialEq)]
pub enum TypedConstant {
    /// `None`.
    None,
    /// `True` or `False`.
    Bool(bool),
    /// An integer of any size, as decimal digits.
    Int(String),
    /// A float.
    Float(f64),
    /// A string.
    Str(String),
    /// A string that holds a surrogate, as its code points (LEX-10).
    StrPoints(Vec<u32>),
    /// A `bytes` literal.
    Bytes(Vec<u8>),
}

impl TypedConstant {
    /// Returns whether the constant is a string with a surrogate outside U+DC80 to U+DCFF,
    /// which standard output's `surrogateescape` cannot write (LEX-10).
    #[must_use]
    pub fn holds_unescapable_surrogate(&self) -> bool {
        matches!(self, Self::StrPoints(points) if points.iter().any(|point| {
            (0xd800..=0xdfff).contains(point) && !(0xdc80..=0xdcff).contains(point)
        }))
    }

    /// Returns the constant's type.
    #[must_use]
    pub fn ty(&self) -> Type {
        match self {
            Self::None => Type::NoneType,
            Self::Bool(_) => Type::Bool,
            Self::Int(_) => Type::Int,
            Self::Float(_) => Type::Float,
            Self::Str(_) | Self::StrPoints(_) => Type::Str,
            Self::Bytes(_) => Type::Bytes,
        }
    }
}
