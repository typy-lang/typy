//! A function, or the module body, after checking.

use typy_diagnostics::Span;

use super::signature::FunctionSignature;
use super::statement::TypedStmt;
use super::symbol::SymbolId;

/// Index of a function in the `TypedProgram`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FunctionId(pub u32);

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
