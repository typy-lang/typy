//! The types of TyIR values.

use std::fmt;

/// The identity of a heap object's type, an index into the type table (detailed design section 10.2).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TypeId(pub u32);

impl TypeId {
    /// `str`.
    pub const STR: Self = Self(1);
    /// An exception object (ADR-0004).
    pub const EXCEPTION: Self = Self(2);
}

/// The type of one TyIR value.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TyirType {
    /// A tagged word: an inline integer, or a pointer to a large one (DAT-01).
    Int,
    /// An IEEE 754 double.
    Float,
    /// A one-bit truth value.
    Bool,
    /// No value: the result of a function that returns `None`.
    Unit,
    /// A pointer to a heap object of the given type.
    Object(TypeId),
}

impl TyirType {
    /// `str`.
    pub const STR: Self = Self::Object(TypeId::STR);
    /// An exception object.
    pub const EXCEPTION: Self = Self::Object(TypeId::EXCEPTION);

    /// Returns true for values that may point to a counted heap object, so they take part in reference counting (ADR-0001).
    #[must_use]
    pub fn is_counted(self) -> bool {
        matches!(self, Self::Int | Self::Object(_))
    }
}

impl fmt::Display for TyirType {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Int => formatter.write_str("int"),
            Self::Float => formatter.write_str("float"),
            Self::Bool => formatter.write_str("bool"),
            Self::Unit => formatter.write_str("unit"),
            Self::Object(TypeId::STR) => formatter.write_str("str"),
            Self::Object(TypeId::EXCEPTION) => formatter.write_str("exception"),
            Self::Object(TypeId(id)) => write!(formatter, "object#{id}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{TyirType, TypeId};

    #[test]
    fn ints_and_objects_are_counted_and_scalars_are_not() {
        assert!(TyirType::Int.is_counted());
        assert!(TyirType::STR.is_counted());
        assert!(!TyirType::Float.is_counted());
        assert!(!TyirType::Bool.is_counted());
        assert!(!TyirType::Unit.is_counted());
    }

    #[test]
    fn types_are_written_by_name() {
        let names: Vec<String> = [
            TyirType::Int,
            TyirType::Float,
            TyirType::Bool,
            TyirType::Unit,
            TyirType::STR,
            TyirType::EXCEPTION,
            TyirType::Object(TypeId(300)),
        ]
        .iter()
        .map(ToString::to_string)
        .collect();
        assert_eq!(
            names,
            [
                "int",
                "float",
                "bool",
                "unit",
                "str",
                "exception",
                "object#300"
            ]
        );
    }
}
