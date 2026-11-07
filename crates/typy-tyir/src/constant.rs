//! Literal values and the string pool.

use std::fmt;

use crate::types::TyirType;

/// Index of a string in the module's string pool.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StringId(pub u32);

/// A literal value.
#[derive(Clone, Debug, PartialEq)]
pub enum Constant {
    /// An integer of any size, as decimal digits with an optional leading `-`.
    Int(String),
    /// A float.
    Float(f64),
    /// A truth value.
    Bool(bool),
    /// No value.
    Unit,
    /// A string from the pool; string constants are never freed.
    Str(StringId),
}

impl Constant {
    /// Returns the constant's type.
    #[must_use]
    pub fn ty(&self) -> TyirType {
        match self {
            Self::Int(_) => TyirType::Int,
            Self::Float(_) => TyirType::Float,
            Self::Bool(_) => TyirType::Bool,
            Self::Unit => TyirType::Unit,
            Self::Str(_) => TyirType::STR,
        }
    }
}

impl fmt::Display for Constant {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Int(digits) => write!(formatter, "int {digits}"),
            Self::Float(value) => write!(formatter, "float {value:?}"),
            Self::Bool(value) => write!(formatter, "bool {value}"),
            Self::Unit => formatter.write_str("unit"),
            Self::Str(StringId(id)) => write!(formatter, "str s{id}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Constant, StringId};
    use crate::types::TyirType;

    #[test]
    fn constants_have_their_types_and_text() {
        let cases = [
            (Constant::Int("-12".to_owned()), TyirType::Int, "int -12"),
            (Constant::Float(1.5), TyirType::Float, "float 1.5"),
            (Constant::Bool(true), TyirType::Bool, "bool true"),
            (Constant::Unit, TyirType::Unit, "unit"),
            (Constant::Str(StringId(3)), TyirType::STR, "str s3"),
        ];
        for (constant, ty, text) in cases {
            assert_eq!(constant.ty(), ty);
            assert_eq!(constant.to_string(), text);
        }
    }
}
