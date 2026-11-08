//! Statements after checking.

use typy_diagnostics::Span;
use typy_source::syntax::BinaryOperator;

use super::expression::TypedExpr;
use super::symbol::SymbolId;
use crate::types::Type;

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

/// What a `for` loop goes over (CST-03, LIB-02). A sequence is read as CPython's list
/// iterator reads it: each step checks the length again, so a list that grows or shrinks in
/// the body changes how many steps there are.
#[derive(Clone, Debug, PartialEq)]
pub enum LoopSource {
    /// The items of a list, or the characters of a `str`.
    Items(TypedExpr),
    /// `reversed(sequence)`: the items from the last to the first.
    Reversed(TypedExpr),
    /// `enumerate(source, start)`: a count from `start`, with each value of `source`.
    Enumerate {
        /// What is counted.
        source: Box<LoopSource>,
        /// The first count.
        start: TypedExpr,
    },
    /// `zip(first, second, ...)`: a tuple of one value from each source, until one ends. With
    /// `strict` true, sources of different lengths raise `ValueError`.
    Zip {
        /// The sources, in order.
        sources: Vec<LoopSource>,
        /// The `strict` argument, when given.
        strict: Option<TypedExpr>,
    },
}

impl LoopSource {
    /// Returns every expression of the source, in the order they are evaluated.
    #[must_use]
    pub fn expressions(&self) -> Vec<&TypedExpr> {
        match self {
            Self::Items(sequence) | Self::Reversed(sequence) => vec![sequence],
            Self::Enumerate { source, start } => {
                let mut expressions = source.expressions();
                expressions.push(start);
                expressions
            }
            Self::Zip { sources, strict } => sources
                .iter()
                .flat_map(Self::expressions)
                .chain(strict)
                .collect(),
        }
    }

    /// Returns the type of what one step gives.
    #[must_use]
    pub fn item_type(&self) -> Type {
        match self {
            Self::Items(sequence) | Self::Reversed(sequence) => match &sequence.ty {
                Type::List(item) => item.as_ref().clone(),
                other => other.clone(),
            },
            Self::Enumerate { source, .. } => Type::Tuple(vec![Type::Int, source.item_type()]),
            Self::Zip { sources, .. } => Type::Tuple(sources.iter().map(Self::item_type).collect()),
        }
    }
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
    /// `for target in source`, over a sequence or a builtin that goes over sequences (CST-03).
    ForEach {
        target: AssignTarget,
        source: LoopSource,
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
