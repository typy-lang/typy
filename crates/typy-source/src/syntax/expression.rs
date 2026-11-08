//! Expressions, literal values and keyword arguments.

use typy_diagnostics::Span;

use super::node_kind::NodeKind;
use super::operator::{BinaryOperator, BoolOperator, CompareOperator, UnaryOperator};
use super::tree::Identifier;

/// A literal value.
#[derive(Clone, Debug, PartialEq)]
pub enum Constant {
    /// `None`.
    None,
    /// `True` or `False`.
    Bool(bool),
    /// An integer of any size, as decimal digits.
    Int(String),
    /// A floating-point literal.
    Float(f64),
    /// An imaginary literal, which TyPy rejects (LEX-08).
    Complex {
        /// The imaginary part.
        imaginary: f64,
    },
    /// A string literal, after escapes and concatenation.
    Str(String),
    /// A bytes literal.
    Bytes(Vec<u8>),
    /// `...`.
    Ellipsis,
}

/// A keyword argument, or `**mapping` when `name` is `None`.
#[derive(Clone, Debug, PartialEq)]
pub struct Keyword {
    /// The keyword, or `None` for `**mapping`.
    pub name: Option<Identifier>,
    /// The value passed.
    pub value: Expr,
    /// The whole argument.
    pub span: Span,
}

/// An expression.
#[derive(Clone, Debug, PartialEq)]
#[allow(
    missing_docs,
    reason = "fields follow the CPython ast node of the same name"
)]
pub enum Expr {
    BoolOp {
        operator: BoolOperator,
        values: Vec<Expr>,
        span: Span,
    },
    BinOp {
        left: Box<Expr>,
        operator: BinaryOperator,
        right: Box<Expr>,
        span: Span,
    },
    UnaryOp {
        operator: UnaryOperator,
        operand: Box<Expr>,
        span: Span,
    },
    IfExp {
        test: Box<Expr>,
        body: Box<Expr>,
        orelse: Box<Expr>,
        span: Span,
    },
    Compare {
        left: Box<Expr>,
        operators: Vec<CompareOperator>,
        comparators: Vec<Expr>,
        span: Span,
    },
    Call {
        function: Box<Expr>,
        arguments: Vec<Expr>,
        keywords: Vec<Keyword>,
        span: Span,
    },
    Constant {
        value: Constant,
        span: Span,
    },
    Subscript {
        value: Box<Expr>,
        index: Box<Expr>,
        span: Span,
    },
    Name {
        id: String,
        span: Span,
    },
    Tuple {
        elements: Vec<Expr>,
        span: Span,
    },
    List {
        elements: Vec<Expr>,
        span: Span,
    },
    Attribute {
        value: Box<Expr>,
        attribute: Identifier,
        span: Span,
    },
    Other {
        kind: NodeKind,
        span: Span,
    },
}

impl Expr {
    /// Returns the span of the whole expression.
    #[must_use]
    pub fn span(&self) -> Span {
        match self {
            Self::BoolOp { span, .. }
            | Self::BinOp { span, .. }
            | Self::UnaryOp { span, .. }
            | Self::IfExp { span, .. }
            | Self::Compare { span, .. }
            | Self::Call { span, .. }
            | Self::Constant { span, .. }
            | Self::Subscript { span, .. }
            | Self::Name { span, .. }
            | Self::Tuple { span, .. }
            | Self::List { span, .. }
            | Self::Attribute { span, .. }
            | Self::Other { span, .. } => *span,
        }
    }
}
