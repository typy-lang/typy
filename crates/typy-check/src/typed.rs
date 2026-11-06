//! The typed program that checking produces and lowering consumes.

use typy_diagnostics::Span;
use typy_source::syntax::{BinaryOperator, BoolOperator, CompareOperator, UnaryOperator};

use crate::builtins::Builtin;
use crate::types::Type;

/// Index of a symbol in the `TypedProgram`'s symbol table.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SymbolId(pub u32);

/// Index of a function in the `TypedProgram`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FunctionId(pub u32);

/// What a symbol names.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SymbolKind {
    /// A parameter of a function.
    Parameter,
    /// A variable of a function, or of the module body.
    Local,
}

/// A name bound in one function or in the module body, with the one type it keeps (TYP-03).
#[derive(Clone, Debug, PartialEq)]
pub struct Symbol {
    /// The symbol's ID.
    pub id: SymbolId,
    /// The Python name.
    pub name: String,
    /// What it names.
    pub kind: SymbolKind,
    /// Its type.
    pub ty: Type,
    /// Where it is first bound.
    pub span: Span,
}

/// A literal value after checking.
#[derive(Clone, Debug, PartialEq)]
pub enum TypedConstant {
    /// `None`.
    None,
    /// `True` or `False`.
    Bool(bool),
    /// An integer of any size, as decimal digits.
    Int(String),
    /// A float.
    Float(f64),
    /// A string.
    Str(String),
}

impl TypedConstant {
    /// Returns the constant's type.
    #[must_use]
    pub fn ty(&self) -> Type {
        match self {
            Self::None => Type::NoneType,
            Self::Bool(_) => Type::Bool,
            Self::Int(_) => Type::Int,
            Self::Float(_) => Type::Float,
            Self::Str(_) => Type::Str,
        }
    }
}

/// One parameter of a function's signature.
#[derive(Clone, Debug, PartialEq)]
pub struct SignatureParameter {
    /// The parameter's name.
    pub name: String,
    /// Its declared type.
    pub ty: Type,
    /// Its default value; a constant in 0.1, so it is evaluated once (EXP-16).
    pub default: Option<TypedConstant>,
}

/// The parameters and return type of a function.
#[derive(Clone, Debug, PartialEq)]
pub struct FunctionSignature {
    /// The parameters, in order.
    pub parameters: Vec<SignatureParameter>,
    /// The return type; `NoneType` for `-> None`.
    pub returns: Type,
}

/// A function, or the module body, after checking.
#[derive(Clone, Debug, PartialEq)]
pub struct TypedFunction {
    /// The function's ID.
    pub id: FunctionId,
    /// The name tracebacks show: the function's name, or `<module>` for the module body.
    pub name: String,
    /// The module the function is defined in.
    pub module: String,
    /// The signature.
    pub signature: FunctionSignature,
    /// One symbol per parameter, in order.
    pub parameters: Vec<SymbolId>,
    /// Every local variable, in the order it is first bound.
    pub locals: Vec<SymbolId>,
    /// The statements.
    pub body: Vec<TypedStmt>,
    /// The whole definition.
    pub span: Span,
}

/// The whole program after checking.
#[derive(Clone, Debug, PartialEq)]
pub struct TypedProgram {
    /// Every symbol, indexed by `SymbolId`.
    pub symbols: Vec<Symbol>,
    /// Every function, indexed by `FunctionId`; the module body is one of them.
    pub functions: Vec<TypedFunction>,
    /// The module body of `__main__`.
    pub entry: FunctionId,
}

impl TypedProgram {
    /// Returns a symbol by its ID.
    #[must_use]
    pub fn symbol(&self, id: SymbolId) -> &Symbol {
        &self.symbols[id.0 as usize]
    }

    /// Returns a function by its ID.
    #[must_use]
    pub fn function(&self, id: FunctionId) -> &TypedFunction {
        &self.functions[id.0 as usize]
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
        targets: Vec<SymbolId>,
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
}
