//! Values: the results of instructions and the parameters of blocks.

use crate::types::TyirType;

/// Index of a value in its function.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ValueId(pub u32);

/// How a value is held (ADR-0001).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ValueRepresentation {
    /// In a register or a slot: `int`, `float`, `bool` and unit.
    Inline,
    /// On the stack; chosen by escape analysis from 0.4 (MEM-03).
    Stack,
    /// A pointer to a counted heap object.
    Heap,
}

/// What is known about one value.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ValueInfo {
    /// Its type.
    pub ty: TyirType,
    /// How it is held.
    pub representation: ValueRepresentation,
}

impl ValueInfo {
    /// Returns the information for a new value of `ty`, held inline unless it is an object.
    #[must_use]
    pub fn of(ty: TyirType) -> Self {
        let representation = match ty {
            TyirType::Object(_) => ValueRepresentation::Heap,
            _ => ValueRepresentation::Inline,
        };
        Self { ty, representation }
    }
}

#[cfg(test)]
mod tests {
    use super::{ValueInfo, ValueRepresentation};
    use crate::types::TyirType;

    #[test]
    fn objects_are_heap_values_and_the_rest_are_inline() {
        assert_eq!(
            ValueInfo::of(TyirType::STR).representation,
            ValueRepresentation::Heap
        );
        for ty in [
            TyirType::Int,
            TyirType::Float,
            TyirType::Bool,
            TyirType::Unit,
        ] {
            assert_eq!(
                ValueInfo::of(ty).representation,
                ValueRepresentation::Inline
            );
        }
    }
}
