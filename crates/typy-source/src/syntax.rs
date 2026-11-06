//! TyPy's syntax tree. Node names follow CPython's `ast` module for Python 3.12.

use std::fmt::{self, Write as _};

use typy_diagnostics::Span;

use crate::source_file::SourceFile;

/// The parsed form of one module.
#[derive(Clone, Debug, PartialEq)]
pub struct SyntaxTree {
    /// The module's statements, in order.
    pub body: Vec<Stmt>,
}

/// An identifier with its location.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Identifier {
    /// The text of the name.
    pub name: String,
    /// Where it appears.
    pub span: Span,
}

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

/// A node kind that the current release does not convert in detail (detailed design section 5.2).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[allow(
    missing_docs,
    reason = "each variant is the CPython ast node of the same name"
)]
pub enum NodeKind {
    AsyncFor,
    AsyncFunctionDef,
    AsyncWith,
    Assert,
    Attribute,
    Await,
    BytesLiteral,
    ClassDef,
    Delete,
    Dict,
    DictComp,
    GeneratorExp,
    Global,
    IpyEscapeCommand,
    JoinedStr,
    Lambda,
    List,
    ListComp,
    Match,
    NamedExpr,
    Nonlocal,
    Raise,
    Set,
    SetComp,
    Slice,
    Starred,
    TemplateStr,
    Try,
    TryStar,
    Tuple,
    TypeAlias,
    With,
    Yield,
    YieldFrom,
}

impl fmt::Display for NodeKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self, formatter)
    }
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
            | Self::Pass { span }
            | Self::Break { span }
            | Self::Continue { span }
            | Self::Other { span, .. } => *span,
        }
    }
}

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

/// `and` or `or`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(
    missing_docs,
    reason = "each variant is the Python operator of the same name"
)]
pub enum BoolOperator {
    And,
    Or,
}

/// A binary arithmetic or bitwise operator.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(
    missing_docs,
    reason = "each variant is the CPython ast operator of the same name"
)]
pub enum BinaryOperator {
    Add,
    Sub,
    Mult,
    MatMult,
    Div,
    Mod,
    Pow,
    LShift,
    RShift,
    BitOr,
    BitXor,
    BitAnd,
    FloorDiv,
}

/// A unary operator.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(
    missing_docs,
    reason = "each variant is the CPython ast operator of the same name"
)]
pub enum UnaryOperator {
    Invert,
    Not,
    UAdd,
    USub,
}

/// A comparison operator.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(
    missing_docs,
    reason = "each variant is the CPython ast operator of the same name"
)]
pub enum CompareOperator {
    Eq,
    NotEq,
    Lt,
    LtE,
    Gt,
    GtE,
    Is,
    IsNot,
    In,
    NotIn,
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
            | Self::Other { span, .. } => *span,
        }
    }
}

/// Returns the tree as indented text, one node per line with its `LINE:COLUMN`, for `--emit=ast`.
#[must_use]
pub fn dump(tree: &SyntaxTree, file: &SourceFile) -> String {
    let mut writer = Dump {
        file,
        text: String::new(),
    };
    writer.line(0, "Module", None);
    for statement in &tree.body {
        writer.statement(1, statement);
    }
    writer.text
}

struct Dump<'file> {
    file: &'file SourceFile,
    text: String,
}

impl Dump<'_> {
    fn line(&mut self, depth: usize, label: &str, span: Option<Span>) {
        let indent = "  ".repeat(depth);
        match span {
            Some(span) => {
                let at = self.file.position(span.start());
                let _ = writeln!(self.text, "{indent}{label} @{}:{}", at.line, at.column);
            }
            None => {
                let _ = writeln!(self.text, "{indent}{label}");
            }
        }
    }

    fn block(&mut self, depth: usize, label: &str, statements: &[Stmt]) {
        if statements.is_empty() {
            return;
        }
        self.line(depth, label, None);
        for statement in statements {
            self.statement(depth + 1, statement);
        }
    }

    #[allow(clippy::too_many_lines, reason = "one arm per statement kind")]
    fn statement(&mut self, depth: usize, statement: &Stmt) {
        let span = Some(statement.span());
        match statement {
            Stmt::FunctionDef {
                name,
                parameters,
                returns,
                body,
                decorators,
                type_parameter_count,
                ..
            } => {
                self.line(depth, &format!("FunctionDef {}", name.name), span);
                for decorator in decorators {
                    self.line(depth + 1, "decorator", None);
                    self.expression(depth + 2, decorator);
                }
                if *type_parameter_count > 0 {
                    self.line(
                        depth + 1,
                        &format!("type_params {type_parameter_count}"),
                        None,
                    );
                }
                for parameter in parameters {
                    let label = format!("Parameter {} {:?}", parameter.name.name, parameter.kind);
                    self.line(depth + 1, &label, Some(parameter.span));
                    if let Some(annotation) = &parameter.annotation {
                        self.expression(depth + 2, annotation);
                    }
                    if let Some(default) = &parameter.default {
                        self.line(depth + 2, "default", None);
                        self.expression(depth + 3, default);
                    }
                }
                if let Some(returns) = returns {
                    self.line(depth + 1, "returns", None);
                    self.expression(depth + 2, returns);
                }
                self.block(depth + 1, "body", body);
            }
            Stmt::Return { value, .. } => {
                self.line(depth, "Return", span);
                if let Some(value) = value {
                    self.expression(depth + 1, value);
                }
            }
            Stmt::Assign { targets, value, .. } => {
                self.line(depth, "Assign", span);
                for target in targets {
                    self.expression(depth + 1, target);
                }
                self.expression(depth + 1, value);
            }
            Stmt::AugAssign {
                target,
                operator,
                value,
                ..
            } => {
                self.line(depth, &format!("AugAssign {operator:?}"), span);
                self.expression(depth + 1, target);
                self.expression(depth + 1, value);
            }
            Stmt::AnnAssign {
                target,
                annotation,
                value,
                ..
            } => {
                self.line(depth, "AnnAssign", span);
                self.expression(depth + 1, target);
                self.expression(depth + 1, annotation);
                if let Some(value) = value {
                    self.expression(depth + 1, value);
                }
            }
            Stmt::For {
                target,
                iter,
                body,
                orelse,
                ..
            } => {
                self.line(depth, "For", span);
                self.expression(depth + 1, target);
                self.expression(depth + 1, iter);
                self.block(depth + 1, "body", body);
                self.block(depth + 1, "orelse", orelse);
            }
            Stmt::While {
                test, body, orelse, ..
            }
            | Stmt::If {
                test, body, orelse, ..
            } => {
                let label = if matches!(statement, Stmt::While { .. }) {
                    "While"
                } else {
                    "If"
                };
                self.line(depth, label, span);
                self.expression(depth + 1, test);
                self.block(depth + 1, "body", body);
                self.block(depth + 1, "orelse", orelse);
            }
            Stmt::Import { names, is_lazy, .. } => {
                let names: Vec<&str> = names.iter().map(|alias| alias.name.name.as_str()).collect();
                let lazy = if *is_lazy { " lazy" } else { "" };
                self.line(depth, &format!("Import{lazy} {}", names.join(", ")), span);
            }
            Stmt::ImportFrom {
                module,
                names,
                level,
                is_lazy,
                ..
            } => {
                let names: Vec<&str> = names.iter().map(|alias| alias.name.name.as_str()).collect();
                let module = module.as_ref().map_or("", |module| module.name.as_str());
                let lazy = if *is_lazy { " lazy" } else { "" };
                let dots = ".".repeat(*level as usize);
                self.line(
                    depth,
                    &format!("ImportFrom{lazy} {dots}{module} {}", names.join(", ")),
                    span,
                );
            }
            Stmt::Expr { value, .. } => {
                self.line(depth, "Expr", span);
                self.expression(depth + 1, value);
            }
            Stmt::Pass { .. } => self.line(depth, "Pass", span),
            Stmt::Break { .. } => self.line(depth, "Break", span),
            Stmt::Continue { .. } => self.line(depth, "Continue", span),
            Stmt::Other { kind, .. } => self.line(depth, &kind.to_string(), span),
        }
    }

    fn expression(&mut self, depth: usize, expression: &Expr) {
        let span = Some(expression.span());
        match expression {
            Expr::BoolOp {
                operator, values, ..
            } => {
                self.line(depth, &format!("BoolOp {operator:?}"), span);
                for value in values {
                    self.expression(depth + 1, value);
                }
            }
            Expr::BinOp {
                left,
                operator,
                right,
                ..
            } => {
                self.line(depth, &format!("BinOp {operator:?}"), span);
                self.expression(depth + 1, left);
                self.expression(depth + 1, right);
            }
            Expr::UnaryOp {
                operator, operand, ..
            } => {
                self.line(depth, &format!("UnaryOp {operator:?}"), span);
                self.expression(depth + 1, operand);
            }
            Expr::IfExp {
                test, body, orelse, ..
            } => {
                self.line(depth, "IfExp", span);
                self.expression(depth + 1, test);
                self.expression(depth + 1, body);
                self.expression(depth + 1, orelse);
            }
            Expr::Compare {
                left,
                operators,
                comparators,
                ..
            } => {
                self.line(depth, &format!("Compare {operators:?}"), span);
                self.expression(depth + 1, left);
                for comparator in comparators {
                    self.expression(depth + 1, comparator);
                }
            }
            Expr::Call {
                function,
                arguments,
                keywords,
                ..
            } => {
                self.line(depth, "Call", span);
                self.expression(depth + 1, function);
                for argument in arguments {
                    self.expression(depth + 1, argument);
                }
                for keyword in keywords {
                    let name = keyword
                        .name
                        .as_ref()
                        .map_or("**", |name| name.name.as_str());
                    self.line(depth + 1, &format!("keyword {name}"), Some(keyword.span));
                    self.expression(depth + 2, &keyword.value);
                }
            }
            Expr::Constant { value, .. } => self.line(depth, &format!("Constant {value:?}"), span),
            Expr::Subscript { value, index, .. } => {
                self.line(depth, "Subscript", span);
                self.expression(depth + 1, value);
                self.expression(depth + 1, index);
            }
            Expr::Name { id, .. } => self.line(depth, &format!("Name {id}"), span),
            Expr::Other { kind, .. } => self.line(depth, &kind.to_string(), span),
        }
    }
}
