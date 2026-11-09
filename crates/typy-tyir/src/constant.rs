//! Literal values and the string pool.

use std::fmt;

use crate::types::{TupleTypeId, TyirType};

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
    /// A `bytes` from the pool, each byte the code point of its value; never freed.
    Bytes(StringId),
    /// The runtime's immortal empty list, the first value of a list slot. Definite assignment
    /// guarantees it is never read, so it is never changed.
    PlaceholderList,
    /// The runtime's immortal empty set, the first value of a set slot, never read either.
    PlaceholderSet,
    /// The runtime's immortal empty dict, the first value of a dict slot, never read either.
    PlaceholderDict,
    /// The address of the descriptor of a tuple type that a list holds, which the runtime
    /// reads to store, count, compare and write its items (ADR-0009).
    TupleDescriptor(TupleTypeId),
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
            Self::Bytes(_) => TyirType::BYTES,
            Self::PlaceholderList => TyirType::LIST,
            Self::PlaceholderSet => TyirType::SET,
            Self::PlaceholderDict => TyirType::DICT,
            Self::TupleDescriptor(_) => TyirType::Bits,
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
            Self::Bytes(StringId(id)) => write!(formatter, "bytes s{id}"),
            Self::PlaceholderList => formatter.write_str("placeholder list"),
            Self::PlaceholderSet => formatter.write_str("placeholder set"),
            Self::PlaceholderDict => formatter.write_str("placeholder dict"),
            Self::TupleDescriptor(TupleTypeId(id)) => write!(formatter, "descriptor tuple#{id}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Constant, StringId};
    use crate::types::{TupleTypeId, TyirType};

    #[test]
    fn constants_have_their_types_and_text() {
        let cases = [
            (Constant::Int("-12".to_owned()), TyirType::Int, "int -12"),
            (Constant::Float(1.5), TyirType::Float, "float 1.5"),
            (Constant::Bool(true), TyirType::Bool, "bool true"),
            (Constant::Unit, TyirType::Unit, "unit"),
            (Constant::Str(StringId(3)), TyirType::STR, "str s3"),
            (Constant::Bytes(StringId(4)), TyirType::BYTES, "bytes s4"),
            (
                Constant::PlaceholderList,
                TyirType::LIST,
                "placeholder list",
            ),
            (Constant::PlaceholderSet, TyirType::SET, "placeholder set"),
            (
                Constant::PlaceholderDict,
                TyirType::DICT,
                "placeholder dict",
            ),
            (
                Constant::TupleDescriptor(TupleTypeId(2)),
                TyirType::Bits,
                "descriptor tuple#2",
            ),
        ];
        for (constant, ty, text) in cases {
            assert_eq!(constant.ty(), ty);
            assert_eq!(constant.to_string(), text);
        }
    }
}
