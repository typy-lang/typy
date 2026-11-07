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
}

impl TypedConstant {
    /// Returns the constant's type.
    #[must_use]
    pub fn ty(&self) -> Type {
        match self {
            Self::None => Type::NoneType,
            Self::Bool(_) => Type::Bool,
            Self::Int(_) => Type::Int,
            Self::Float(_) => Type::Float,
            Self::Str(_) => Type::Str,
        }
    }
}
