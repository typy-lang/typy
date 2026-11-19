//! A function, or the module body, after checking.

use typy_diagnostics::Span;

use super::signature::FunctionSignature;
use super::statement::TypedStmt;
use super::symbol::SymbolId;
use crate::types::Type;

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
    /// The name that tells it from every other function of its module: the name, or for a
    /// method the class's name, a dot and the method's name, as Python's `__qualname__`.
    pub qualified_name: String,
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
    /// Whether it is a generic function, whose copies run in its place; it holds no body
    /// (GEN-03, ADR-0023).
    pub generic: bool,
    /// The variables an assignment expression assigns, which no statement shows (EXP-13).
    pub named_targets: Vec<SymbolId>,
    /// The types of a generator function, whose call makes a generator, or `None` for any
    /// other function (FUN-07, ADR-0063).
    pub generator: Option<GeneratorTypes>,
}

/// What a generator function's `yield` gives, what `send` passes to it and what its
/// `return` gives, from its annotation (FUN-07).
#[derive(Clone, Debug, PartialEq)]
pub struct GeneratorTypes {
    /// The type of each item.
    pub yields: Type,
    /// The type `send` passes, `None` for an `Iterator` or an `Iterable`.
    pub sends: Type,
    /// The type of the result, `None` for an `Iterator` or an `Iterable`.
    pub returns: Type,
}
