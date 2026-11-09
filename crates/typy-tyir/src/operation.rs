//! The operations of TyIR instructions.

use std::fmt;

use crate::constant::Constant;
use crate::runtime::RuntimeFunction;
use crate::slot::SlotId;
use crate::value::ValueId;

/// An arithmetic or bitwise operation on two `int` values that cannot raise.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[allow(
    missing_docs,
    reason = "each variant is the Python operator of the same name"
)]
pub enum IntOperator {
    Add,
    Subtract,
    Multiply,
    BitAnd,
    BitOr,
    BitXor,
}

/// An operation on one `int`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[allow(
    missing_docs,
    reason = "each variant is the Python operator of the same name"
)]
pub enum IntUnaryOperator {
    Negate,
    Invert,
}

/// An arithmetic operation on two `float` values that cannot raise.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[allow(
    missing_docs,
    reason = "each variant is the Python operator of the same name"
)]
pub enum FloatOperator {
    Add,
    Subtract,
    Multiply,
}

/// An operation on two `bool` values.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[allow(
    missing_docs,
    reason = "each variant is the Python operator of the same name"
)]
pub enum BoolOperator {
    And,
    Or,
    Xor,
}

/// A value comparison.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[allow(
    missing_docs,
    reason = "each variant is the Python operator of the same name"
)]
pub enum Comparison {
    Equal,
    NotEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
}

impl Comparison {
    /// Returns the number the runtime's comparison functions take for this comparison.
    #[must_use]
    pub const fn code(self) -> i64 {
        match self {
            Self::Equal => 0,
            Self::NotEqual => 1,
            Self::Less => 2,
            Self::LessEqual => 3,
            Self::Greater => 4,
            Self::GreaterEqual => 5,
        }
    }
}

/// The type a comparison or a truth test works on.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[allow(
    missing_docs,
    reason = "each variant is the TyIR type of the same name"
)]
pub enum ScalarKind {
    Int,
    Float,
    Bool,
    Str,
}

/// What one instruction does. An operation cannot raise; a call that may raise is an `invoke` terminator.
#[derive(Clone, Debug, PartialEq)]
pub enum Operation {
    /// A literal value.
    Constant(Constant),
    /// The value a slot holds, borrowed from the slot.
    Load(SlotId),
    /// Stores a value in a slot; the slot takes over the reference.
    Store {
        /// The slot.
        slot: SlotId,
        /// The value.
        value: ValueId,
    },
    /// An `int` operation that cannot raise.
    IntBinary {
        /// The operator.
        operator: IntOperator,
        /// The left operand.
        left: ValueId,
        /// The right operand.
        right: ValueId,
    },
    /// An operation on one `int`.
    IntUnary {
        /// The operator.
        operator: IntUnaryOperator,
        /// The operand.
        operand: ValueId,
    },
    /// A `float` operation that cannot raise.
    FloatBinary {
        /// The operator.
        operator: FloatOperator,
        /// The left operand.
        left: ValueId,
        /// The right operand.
        right: ValueId,
    },
    /// `-x` on a `float`.
    FloatNegate(ValueId),
    /// An operation on two `bool` values.
    BoolBinary {
        /// The operator.
        operator: BoolOperator,
        /// The left operand.
        left: ValueId,
        /// The right operand.
        right: ValueId,
    },
    /// `not x` on a `bool`.
    BoolNot(ValueId),
    /// Compares two values of one scalar kind.
    Compare {
        /// The kind both operands have.
        kind: ScalarKind,
        /// The comparison.
        comparison: Comparison,
        /// The left operand.
        left: ValueId,
        /// The right operand.
        right: ValueId,
    },
    /// The truth value of a value (EXP-10).
    IsTrue {
        /// The kind of the value.
        kind: ScalarKind,
        /// The value.
        value: ValueId,
    },
    /// Calls a runtime function that cannot raise.
    CallRuntime {
        /// The function.
        function: RuntimeFunction,
        /// The arguments, borrowed.
        arguments: Vec<ValueId>,
    },
    /// Calls a program function that cannot raise.
    Call {
        /// The function's index in the module.
        function: crate::function::FunctionId,
        /// The arguments, borrowed.
        arguments: Vec<ValueId>,
    },
    /// A tuple of these values, which it takes over (DAT-08).
    MakeTuple {
        /// The tuple's type.
        ty: crate::types::TupleTypeId,
        /// One value for each item, in order.
        items: Vec<ValueId>,
    },
    /// The item of a tuple at a position, borrowed from the tuple.
    TupleItem {
        /// The tuple.
        tuple: ValueId,
        /// The position, from 0.
        index: u32,
    },
    /// The eight bytes a list stores for a value: an `int` word, the bits of a `float`, a
    /// `bool` as 0 or 1, or a pointer (MEM-02).
    ToBits(ValueId),
    /// The value of type `ty` that eight bytes from a list hold, borrowed from the list.
    FromBits {
        /// The bytes.
        bits: ValueId,
        /// The value's type.
        ty: crate::types::TyirType,
    },
    /// A `T | None` that holds `value`, of the optional's base (ADR-0011).
    Wrap {
        /// The value.
        value: ValueId,
        /// The optional type.
        ty: crate::types::TyirType,
    },
    /// The value an optional holds, which is not `None`.
    Unwrap(ValueId),
    /// Whether an optional value is `None`.
    IsNone(ValueId),
    /// Adds one to the count of a counted value (ADR-0001).
    Increment(ValueId),
    /// Takes one from the count of a counted value, and frees it at zero.
    Decrement(ValueId),
}

impl fmt::Display for IntOperator {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Add => "add",
            Self::Subtract => "sub",
            Self::Multiply => "mul",
            Self::BitAnd => "and",
            Self::BitOr => "or",
            Self::BitXor => "xor",
        })
    }
}

impl fmt::Display for FloatOperator {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Add => "add",
            Self::Subtract => "sub",
            Self::Multiply => "mul",
        })
    }
}

impl fmt::Display for BoolOperator {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::And => "and",
            Self::Or => "or",
            Self::Xor => "xor",
        })
    }
}

impl fmt::Display for Comparison {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Equal => "eq",
            Self::NotEqual => "ne",
            Self::Less => "lt",
            Self::LessEqual => "le",
            Self::Greater => "gt",
            Self::GreaterEqual => "ge",
        })
    }
}

impl fmt::Display for ScalarKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Int => "int",
            Self::Float => "float",
            Self::Bool => "bool",
            Self::Str => "str",
        })
    }
}

#[cfg(test)]
mod tests {
    use super::Comparison;

    #[test]
    fn comparison_codes_are_distinct_and_dense() {
        let codes: Vec<i64> = [
            Comparison::Equal,
            Comparison::NotEqual,
            Comparison::Less,
            Comparison::LessEqual,
            Comparison::Greater,
            Comparison::GreaterEqual,
        ]
        .iter()
        .map(|comparison| comparison.code())
        .collect();
        assert_eq!(codes, [0, 1, 2, 3, 4, 5]);
    }
}
