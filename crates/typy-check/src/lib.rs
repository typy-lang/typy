//! Resolves names, checks types and the subset rules of a TyPy program (the Checking context).
#![forbid(unsafe_code)]

mod body;
mod builtin;
mod check;
mod codec;
mod method;
mod subset;
mod typed;
mod types;
mod unprovided;

pub use builtin::Builtin;
pub use check::TypeChecker;
pub use method::Method;
pub use typed::{
    Argument, AssignTarget, Attribute, ClassId, FunctionId, FunctionSignature, Generator,
    LoopSource, SignatureParameter, Symbol, SymbolId, SymbolKind, TypedClass, TypedConstant,
    TypedExpr, TypedExprKind, TypedFunction, TypedProgram, TypedStmt, dump,
};
pub use types::{Type, TypeObjectOf, View};
