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
    /// `sorted`, `sum`, `any`, `all`, or `min` or `max` of one iterable, with the one optional
    /// value the builtin takes: `reverse`, `start` or `default` (LIB-02).
    IterableBuiltin {
        builtin: Builtin,
        iterable: Box<TypedExpr>,
        option: Option<Box<TypedExpr>>,
    },
    /// A `tuple[T, ...]` made from a fixed tuple whose items are all of type `T`, or from a new
    /// list that `tuple(source)` built (DAT-14).
    TupleOf(Box<TypedExpr>),
    /// `list(source)`: a new list of every value the source gives, in order (LIB-02).
    ListOf(Box<super::statement::LoopSource>),
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
    /// A tuple display, one value per item (DAT-08).
    Tuple(Vec<TypedExpr>),
    /// A list display, one value per item, in order (DAT-07).
    List(Vec<TypedExpr>),
    /// `list[index]`, counted from the end when negative (DAT-11).
    ListItem {
        list: Box<TypedExpr>,
        index: Box<TypedExpr>,
    },
    /// `value[lower:upper:step]` on a `str` or a list, each bound optional (DAT-12).
    Slice {
        value: Box<TypedExpr>,
        lower: Option<Box<TypedExpr>>,
        upper: Option<Box<TypedExpr>>,
        step: Option<Box<TypedExpr>>,
    },
    /// A method of a built-in type called on `receiver` (LIB-03).
    Method {
        receiver: Box<TypedExpr>,
        method: crate::method::Method,
        arguments: Vec<TypedExpr>,
    },
    /// A new tuple of the items of `tuple` at positions known when compiling (DAT-12).
    TupleSlice {
        tuple: Box<TypedExpr>,
        positions: Vec<usize>,
    },
    /// The item of a tuple at a position known when compiling, counted from the start.
    TupleItem {
        tuple: Box<TypedExpr>,
        index: usize,
    },
}
