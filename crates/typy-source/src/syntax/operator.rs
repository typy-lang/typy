//! The operators of Python's expressions.

/// `and` or `or`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(
    missing_docs,
    reason = "each variant is the Python operator of the same name"
)]
pub enum BoolOperator {
    And,
    Or,
}

/// A binary arithmetic or bitwise operator.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(
    missing_docs,
    reason = "each variant is the CPython ast operator of the same name"
)]
pub enum BinaryOperator {
    Add,
    Sub,
    Mult,
    MatMult,
    Div,
    Mod,
    Pow,
    LShift,
    RShift,
    BitOr,
    BitXor,
    BitAnd,
    FloorDiv,
}

/// A unary operator.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(
    missing_docs,
    reason = "each variant is the CPython ast operator of the same name"
)]
pub enum UnaryOperator {
    Invert,
    Not,
    UAdd,
    USub,
}

/// A comparison operator.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(
    missing_docs,
    reason = "each variant is the CPython ast operator of the same name"
)]
pub enum CompareOperator {
    Eq,
    NotEq,
    Lt,
    LtE,
    Gt,
    GtE,
    Is,
    IsNot,
    In,
    NotIn,
}
