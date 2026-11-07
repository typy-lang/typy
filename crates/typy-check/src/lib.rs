//! Resolves names, checks types and the subset rules of a TyPy program (the Checking context).
#![forbid(unsafe_code)]

mod body;
mod builtin;
mod check;
mod subset;
mod typed;
mod types;
mod unprovided;

pub use builtin::Builtin;
pub use check::TypeChecker;
pub use typed::{
    Argument, FunctionId, FunctionSignature, SignatureParameter, Symbol, SymbolId, SymbolKind,
    TypedConstant, TypedExpr, TypedExprKind, TypedFunction, TypedProgram, TypedStmt,
};
pub use types::Type;
