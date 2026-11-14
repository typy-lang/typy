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

/// One type parameter of a `def`, a `class` or a `type` statement (GEN-01).
#[derive(Clone, Debug, PartialEq)]
pub struct TypeParameter {
    /// The parameter's name.
    pub name: Identifier,
    /// What it stands for.
    pub kind: TypeParameterKind,
    /// Its bound, or a tuple of its constraints, written after a colon.
    pub bound: Option<Expr>,
    /// Its default, written after `=` (Python 3.13).
    pub default: Option<Expr>,
    /// The whole parameter.
    pub span: Span,
}

/// What a type parameter stands for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TypeParameterKind {
    /// `T`: one type.
    Type,
    /// `*Ts`: any number of types.
    TypeTuple,
    /// `**P`: the parameters of a callable.
    Parameters,
}

/// One name in an `import` statement.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Alias {
    /// The imported name, dotted for `import a.b`.
    pub name: Identifier,
    /// The name after `as`, if any.
    pub as_name: Option<Identifier>,
}

/// One item of a `with` statement: a context manager, and the target `as` binds.
#[derive(Clone, Debug, PartialEq)]
pub struct WithItem {
    /// The expression that gives the context manager.
    pub context: Expr,
    /// The target after `as`, if any.
    pub target: Option<Expr>,
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
        type_parameters: Vec<TypeParameter>,
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
        type_parameters: Vec<TypeParameter>,
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
    /// `with item, ...: body`.
    With {
        items: Vec<WithItem>,
        body: Vec<Stmt>,
        span: Span,
    },
    /// `del target, ...`.
    Delete {
        targets: Vec<Expr>,
        span: Span,
    },
    /// `assert test` or `assert test, message`.
    Assert {
        test: Expr,
        message: Option<Expr>,
        span: Span,
    },
    /// `type name = value` or `type name[T, ...] = value`, a type alias whose value is an
    /// annotation (SMT-10).
    TypeAlias {
        name: Identifier,
        type_parameters: Vec<TypeParameter>,
        value: Expr,
        span: Span,
    },
    /// `global name, ...`: the names are the module's variables in this function (SMT-08).
    Global {
        names: Vec<Identifier>,
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
        match self {
            Self::Try {
                body,
                handlers,
                orelse,
                finalbody,
                ..
            } => std::iter::once(body.as_slice())
                .chain(handlers.iter().map(|handler| handler.body.as_slice()))
                .chain([orelse.as_slice(), finalbody.as_slice()])
                .collect(),
            _ => Vec::new(),
        }
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
            | Self::With { span, .. }
            | Self::Delete { span, .. }
            | Self::Try { span, .. }
            | Self::Global { span, .. }
            | Self::TypeAlias { span, .. }
            | Self::Pass { span }
            | Self::Break { span }
            | Self::Continue { span }
            | Self::Other { span, .. } => *span,
        }
    }
}
