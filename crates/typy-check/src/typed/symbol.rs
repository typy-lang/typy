//! Symbols: the names a function or the module body binds.

use typy_diagnostics::Span;

use crate::types::Type;

/// Index of a symbol in the `TypedProgram`'s symbol table.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SymbolId(pub u32);

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
