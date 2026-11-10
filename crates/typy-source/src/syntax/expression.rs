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

/// One `for target in iter if condition ...` clause of a comprehension, as CPython's
/// `ast.comprehension`.
#[derive(Clone, Debug, PartialEq)]
pub struct Comprehension {
    /// What each value is bound to.
    pub target: Expr,
    /// What the clause goes over.
    pub iter: Expr,
    /// The `if` conditions, in order.
    pub ifs: Vec<Expr>,
    /// True for `async for`.
    pub is_async: bool,
    /// Where the clause is.
    pub span: Span,
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
    /// An annotation written as a string, such as `"Node | None"`, with the expression the
    /// string holds; only annotations are read this way.
    Quoted {
        annotation: Box<Expr>,
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
    /// A dict display, `{k: v}`, as CPython's `ast.Dict`: each key with its value, and a key of
    /// `None` for `**mapping`.
    Dict {
        keys: Vec<Option<Expr>>,
        values: Vec<Expr>,
        span: Span,
    },
    /// `[element for ...]`.
    ListComp {
        element: Box<Expr>,
        generators: Vec<Comprehension>,
        span: Span,
    },
    /// `{element for ...}`.
    SetComp {
        element: Box<Expr>,
        generators: Vec<Comprehension>,
        span: Span,
    },
    /// `{key: value for ...}`.
    DictComp {
        key: Box<Expr>,
        value: Box<Expr>,
        generators: Vec<Comprehension>,
        span: Span,
    },
    /// A set display, `{a, b}`; `{}` is an empty dict, so a set display always has items.
    Set {
        elements: Vec<Expr>,
        span: Span,
    },
    Attribute {
        value: Box<Expr>,
        attribute: Identifier,
        span: Span,
    },
    Slice {
        lower: Option<Box<Expr>>,
        upper: Option<Box<Expr>>,
        step: Option<Box<Expr>>,
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
            | Self::Quoted { span, .. }
            | Self::Tuple { span, .. }
            | Self::List { span, .. }
            | Self::Set { span, .. }
            | Self::Dict { span, .. }
            | Self::ListComp { span, .. }
            | Self::SetComp { span, .. }
            | Self::DictComp { span, .. }
            | Self::Attribute { span, .. }
            | Self::Slice { span, .. }
            | Self::Other { span, .. } => *span,
        }
    }
}
