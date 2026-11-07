//! Expressions after checking, each with its type.

use typy_diagnostics::Span;
use typy_source::syntax::{BinaryOperator, BoolOperator, CompareOperator, UnaryOperator};

use super::constant::TypedConstant;
use super::function::FunctionId;
use super::symbol::SymbolId;
use crate::builtin::Builtin;
use crate::types::Type;

/// An expression after checking, with its type.
#[derive(Clone, Debug, PartialEq)]
pub struct TypedExpr {
    /// What the expression does.
    pub kind: TypedExprKind,
    /// Its type.
    pub ty: Type,
    /// Where it is.
    pub span: Span,
}

/// One argument of a call, in source order, with the parameter it binds.
#[derive(Clone, Debug, PartialEq)]
pub struct Argument {
    /// The index of the parameter in the callee's signature.
    pub parameter: usize,
    /// The value.
    pub value: TypedExpr,
}

/// What an expression does.
#[derive(Clone, Debug, PartialEq)]
#[allow(
    missing_docs,
    reason = "each variant is the Python expression of the same name"
)]
pub enum TypedExprKind {
    Constant(TypedConstant),
    Local(SymbolId),
    /// `__name__`, which is `"__main__"` in the main module (STD-06).
    ModuleName,
    Binary {
        left: Box<TypedExpr>,
        operator: BinaryOperator,
        right: Box<TypedExpr>,
    },
    Unary {
        operator: UnaryOperator,
        operand: Box<TypedExpr>,
    },
    BoolOp {
        operator: BoolOperator,
        values: Vec<TypedExpr>,
    },
    Compare {
        left: Box<TypedExpr>,
        operators: Vec<CompareOperator>,
        comparators: Vec<TypedExpr>,
    },
    IfExp {
        test: Box<TypedExpr>,
        body: Box<TypedExpr>,
        orelse: Box<TypedExpr>,
    },
    /// A call of a program function. Arguments are in source order, which is evaluation order (EXP-01); missing ones take their defaults.
    Call {
        function: FunctionId,
        arguments: Vec<Argument>,
    },
    /// `print(values, sep=sep, end=end)`; `sep` and `end` are present only when written.
    Print {
        values: Vec<TypedExpr>,
        sep: Option<Box<TypedExpr>>,
        end: Option<Box<TypedExpr>>,
    },
    /// A call of any other builtin of 0.1, with positional arguments only.
    Builtin {
        builtin: Builtin,
        arguments: Vec<TypedExpr>,
    },
    /// `value[index]` on a `str`.
    Index {
        value: Box<TypedExpr>,
        index: Box<TypedExpr>,
    },
}
