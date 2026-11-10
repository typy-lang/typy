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
    /// A `list`, whatever its item type; the item type lives in the checked program.
    pub const LIST: Self = Self(4);
    /// `bytes`, which the runtime stores as a `str` object of one byte per code point.
    pub const BYTES: Self = Self(6);
    /// A `set`, whatever its item type.
    pub const SET: Self = Self(7);
    /// A `dict`, whatever its key and value types.
    pub const DICT: Self = Self(8);
    /// A view of a dict's keys, values or items.
    pub const DICT_VIEW: Self = Self(9);
    /// The first ID of the program's classes; class `n` of the module is this plus `n`.
    const FIRST_CLASS: u32 = 1 << 16;

    /// Returns the type of the instances of class `index` of the module (CLS-01).
    #[must_use]
    pub const fn class(index: u32) -> Self {
        Self(Self::FIRST_CLASS + index)
    }

    /// Returns the index of the class whose instances have this type, if it is a class.
    #[must_use]
    pub const fn class_index(self) -> Option<u32> {
        self.0.checked_sub(Self::FIRST_CLASS)
    }
}

/// The number of a tuple type in its module's `TupleTypes`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TupleTypeId(pub u32);

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
    /// A tuple, held as a value with its items in place; `TupleTypes` gives the item types.
    Tuple(TupleTypeId),
    /// The eight bytes of a list item, as the runtime stores it; never counted (MEM-02).
    Bits,
    /// A `T | None`, held in a value its base never takes (ADR-0011).
    Optional(OptionalBase),
}

/// What an optional value holds when it is not `None`, which decides how it holds `None`
/// (ADR-0011).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum OptionalBase {
    /// An `int` word, or 0.
    Int,
    /// The bits of a `float`, or the reserved signalling NaN.
    Float,
    /// A `bool` widened to a byte, or 2.
    Bool,
    /// An object's address, or null.
    Object(TypeId),
    /// A tuple, held as this two-item tuple type of a presence flag and the tuple.
    Tuple(TupleTypeId),
}

impl TyirType {
    /// `str`.
    pub const STR: Self = Self::Object(TypeId::STR);
    /// An exception object.
    pub const EXCEPTION: Self = Self::Object(TypeId::EXCEPTION);
    /// A list.
    pub const LIST: Self = Self::Object(TypeId::LIST);
    /// `bytes`.
    pub const BYTES: Self = Self::Object(TypeId::BYTES);
    /// A set.
    pub const SET: Self = Self::Object(TypeId::SET);
    /// A dict.
    pub const DICT: Self = Self::Object(TypeId::DICT);
    /// A view of a dict.
    pub const VIEW: Self = Self::Object(TypeId::DICT_VIEW);
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
            Self::Object(TypeId::LIST) => formatter.write_str("list"),
            Self::Object(TypeId::SET) => formatter.write_str("set"),
            Self::Object(TypeId::DICT) => formatter.write_str("dict"),
            Self::Object(TypeId::DICT_VIEW) => formatter.write_str("view"),
            Self::Bits => formatter.write_str("bits"),
            Self::Object(id) if id.class_index().is_some() => {
                write!(formatter, "class#{}", id.class_index().unwrap_or(0))
            }
            Self::Object(TypeId(id)) => write!(formatter, "object#{id}"),
            Self::Tuple(TupleTypeId(id)) => write!(formatter, "tuple#{id}"),
            Self::Optional(base) => {
                let value = match base {
                    OptionalBase::Int => Self::Int,
                    OptionalBase::Float => Self::Float,
                    OptionalBase::Bool => Self::Bool,
                    OptionalBase::Object(id) => Self::Object(*id),
                    OptionalBase::Tuple(pair) => Self::Tuple(*pair),
                };
                write!(formatter, "optional {value}")
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{TupleTypeId, TyirType, TypeId};

    #[test]
    fn types_are_written_by_name() {
        let names: Vec<String> = [
            TyirType::Int,
            TyirType::Float,
            TyirType::Bool,
            TyirType::Unit,
            TyirType::STR,
            TyirType::EXCEPTION,
            TyirType::SET,
            TyirType::DICT,
            TyirType::VIEW,
            TyirType::Object(TypeId(300)),
            TyirType::Tuple(TupleTypeId(4)),
            TyirType::Optional(super::OptionalBase::Int),
            TyirType::Optional(super::OptionalBase::Tuple(TupleTypeId(2))),
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
                "set",
                "dict",
                "view",
                "object#300",
                "tuple#4",
                "optional int",
                "optional tuple#2"
            ]
        );
    }
}
