//! Statements after checking.

use typy_diagnostics::Span;
use typy_source::syntax::BinaryOperator;

use super::expression::TypedExpr;
use super::symbol::SymbolId;

/// Where an assignment stores its value: a variable, or a tuple of targets that the value is
/// unpacked into, item by item (SMT-02).
#[derive(Clone, Debug, PartialEq)]
pub enum AssignTarget {
    /// A variable.
    Name(SymbolId),
    /// One target for each item of a tuple value.
    Tuple(Vec<AssignTarget>),
    /// An item of a list, `list[index] = value`; the list and the index are evaluated after the
    /// value, as CPython does (DAT-07).
    Item {
        /// The list.
        list: Box<TypedExpr>,
        /// The position, counted from the end when negative.
        index: Box<TypedExpr>,
    },
}

/// A statement after checking.
#[derive(Clone, Debug, PartialEq)]
#[allow(
    missing_docs,
    reason = "each variant is the Python statement of the same name"
)]
pub enum TypedStmt {
    /// `a = b = value`: the value is evaluated once, then bound to each target in order.
    Assign {
        targets: Vec<AssignTarget>,
        value: TypedExpr,
        span: Span,
    },
    /// `target op= value`: an item target evaluates its list and index once, then reads the item
    /// before the value is evaluated, as CPython does. A list target with `+=` or `*=` changes
    /// the list in place.
    AugAssign {
        target: AssignTarget,
        operator: BinaryOperator,
        value: TypedExpr,
        span: Span,
    },
    Expr {
        value: TypedExpr,
        span: Span,
    },
    Return {
        value: Option<TypedExpr>,
        span: Span,
    },
    If {
        test: TypedExpr,
        body: Vec<TypedStmt>,
        orelse: Vec<TypedStmt>,
        span: Span,
    },
    While {
        test: TypedExpr,
        body: Vec<TypedStmt>,
        orelse: Vec<TypedStmt>,
        span: Span,
    },
    /// `for target in range(start, stop, step)`; the arguments are evaluated once, in order.
    /// `for target in iterable`, over the items of a list (CST-03).
    ForEach {
        target: AssignTarget,
        iterable: TypedExpr,
        body: Vec<TypedStmt>,
        orelse: Vec<TypedStmt>,
        span: Span,
    },
    ForRange {
        target: SymbolId,
        start: TypedExpr,
        stop: TypedExpr,
        step: TypedExpr,
        body: Vec<TypedStmt>,
        orelse: Vec<TypedStmt>,
        span: Span,
    },
    Pass {
        span: Span,
    },
    Break {
        span: Span,
    },
    Continue {
        span: Span,
    },
}
