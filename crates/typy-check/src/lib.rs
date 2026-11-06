//! Resolves names, checks types and the subset rules of a TyPy program (the Checking context).
#![forbid(unsafe_code)]

mod body;
mod builtins;
mod check;
mod subset;
mod typed;
mod types;

pub use builtins::Builtin;
pub use check::TypeChecker;
pub use typed::{
    Argument, FunctionId, FunctionSignature, SignatureParameter, Symbol, SymbolId, SymbolKind,
    TypedConstant, TypedExpr, TypedExprKind, TypedFunction, TypedProgram, TypedStmt,
};
pub use types::Type;

#[cfg(test)]
mod tests;
