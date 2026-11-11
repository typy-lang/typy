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
    /// A variable of a function.
    Local,
    /// A variable of a module, bound at module level and kept for the whole run: module code
    /// reads it as a local, and a function reads it, or assigns it after `global` (SMT-08).
    ModuleVariable,
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
