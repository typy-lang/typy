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

/// One `except` clause of a `try` statement.
#[derive(Clone, Debug, PartialEq)]
pub struct ExceptHandler {
    /// What the clause catches: a class, a tuple of classes, or nothing for a bare `except`.
    pub class: Option<Expr>,
    /// The name after `as`, if any.
    pub name: Option<Identifier>,
    /// The clause's body.
    pub body: Vec<Stmt>,
    /// The whole clause.
    pub span: Span,
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
    /// `try` with its `except` clauses, its `else` block and its `finally` block.
    Try {
        body: Vec<Stmt>,
        handlers: Vec<ExceptHandler>,
        orelse: Vec<Stmt>,
        finalbody: Vec<Stmt>,
        span: Span,
    },
    /// `assert test` or `assert test, message`.
    Assert {
        test: Expr,
        message: Option<Expr>,
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
    /// Returns the blocks of a `try` statement in source order: its body, the body of each
    /// `except` clause, its `else` block and its `finally` block; none for another statement.
    #[must_use]
    pub fn try_blocks(&self) -> Vec<&[Self]> {
        let Self::Try {
            body,
            handlers,
            orelse,
            finalbody,
            ..
        } = self
        else {
            return Vec::new();
        };
        let mut blocks = vec![body.as_slice()];
        blocks.extend(handlers.iter().map(|handler| handler.body.as_slice()));
        blocks.push(orelse);
        blocks.push(finalbody);
        blocks
    }

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
            | Self::Assert { span, .. }
            | Self::Try { span, .. }
            | Self::Pass { span }
            | Self::Break { span }
            | Self::Continue { span }
            | Self::Other { span, .. } => *span,
        }
    }
}
