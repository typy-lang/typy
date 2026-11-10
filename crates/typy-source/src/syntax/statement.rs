//! Statements, function parameters and import names.

use typy_diagnostics::Span;

use super::expression::Expr;
use super::node_kind::NodeKind;
use super::operator::BinaryOperator;
use super::tree::Identifier;

/// How a parameter may be passed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParameterKind {
    /// Before `/`.
    PositionalOnly,
    /// Positional or by keyword.
    Normal,
    /// `*args`.
    VarPositional,
    /// After `*` or `*args`.
    KeywordOnly,
    /// `**kwargs`.
    VarKeyword,
}

/// One parameter of a function definition.
#[derive(Clone, Debug, PartialEq)]
pub struct Parameter {
    /// The parameter's name.
    pub name: Identifier,
    /// Its annotation, if any.
    pub annotation: Option<Expr>,
    /// Its default value, if any.
    pub default: Option<Expr>,
    /// How it may be passed.
    pub kind: ParameterKind,
    /// The whole parameter.
    pub span: Span,
}

/// One name in an `import` statement.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Alias {
    /// The imported name, dotted for `import a.b`.
    pub name: Identifier,
    /// The name after `as`, if any.
    pub as_name: Option<Identifier>,
}

/// A statement.
#[derive(Clone, Debug, PartialEq)]
#[allow(
    missing_docs,
    reason = "fields follow the CPython ast node of the same name"
)]
pub enum Stmt {
    FunctionDef {
        name: Identifier,
        parameters: Vec<Parameter>,
        returns: Option<Expr>,
        body: Vec<Stmt>,
        decorators: Vec<Expr>,
        type_parameter_count: usize,
        span: Span,
    },
    /// `class name(bases): body`; `keyword_count` counts keyword arguments such as
    /// `metaclass=...`.
    ClassDef {
        name: Identifier,
        bases: Vec<Expr>,
        keyword_count: usize,
        body: Vec<Stmt>,
        decorators: Vec<Expr>,
        type_parameter_count: usize,
        span: Span,
    },
    Return {
        value: Option<Expr>,
        span: Span,
    },
    Assign {
        targets: Vec<Expr>,
        value: Expr,
        span: Span,
    },
    AugAssign {
        target: Expr,
        operator: BinaryOperator,
        value: Expr,
        span: Span,
    },
    AnnAssign {
        target: Expr,
        annotation: Expr,
        value: Option<Expr>,
        simple: bool,
        span: Span,
    },
    For {
        target: Expr,
        iter: Expr,
        body: Vec<Stmt>,
        orelse: Vec<Stmt>,
        span: Span,
    },
    While {
        test: Expr,
        body: Vec<Stmt>,
        orelse: Vec<Stmt>,
        span: Span,
    },
    If {
        test: Expr,
        body: Vec<Stmt>,
        orelse: Vec<Stmt>,
        span: Span,
    },
    Import {
        names: Vec<Alias>,
        is_lazy: bool,
        span: Span,
    },
    ImportFrom {
        module: Option<Identifier>,
        names: Vec<Alias>,
        level: u32,
        is_lazy: bool,
        span: Span,
    },
    Expr {
        value: Expr,
        span: Span,
    },
    /// `raise`, `raise exception` or `raise exception from cause`.
    Raise {
        exception: Option<Expr>,
        cause: Option<Expr>,
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
    Other {
        kind: NodeKind,
        span: Span,
    },
}

impl Stmt {
    /// Returns the span of the whole statement.
    #[must_use]
    pub fn span(&self) -> Span {
        match self {
            Self::FunctionDef { span, .. }
            | Self::ClassDef { span, .. }
            | Self::Return { span, .. }
            | Self::Assign { span, .. }
            | Self::AugAssign { span, .. }
            | Self::AnnAssign { span, .. }
            | Self::For { span, .. }
            | Self::While { span, .. }
            | Self::If { span, .. }
            | Self::Import { span, .. }
            | Self::ImportFrom { span, .. }
            | Self::Expr { span, .. }
            | Self::Raise { span, .. }
            | Self::Pass { span }
            | Self::Break { span }
            | Self::Continue { span }
            | Self::Other { span, .. } => *span,
        }
    }
}
