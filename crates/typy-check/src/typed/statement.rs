//! Statements after checking.

use typy_diagnostics::Span;
use typy_source::syntax::BinaryOperator;

use super::expression::TypedExpr;
use super::symbol::SymbolId;

/// A statement after checking.
/// Where an assignment stores its value: a variable, or a tuple of targets that the value is
/// unpacked into, item by item (SMT-02).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AssignTarget {
    /// A variable.
    Name(SymbolId),
    /// One target for each item of a tuple value.
    Tuple(Vec<AssignTarget>),
}

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
    AugAssign {
        target: SymbolId,
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
