//! The parsed form of one module, and identifiers.

use typy_diagnostics::Span;

use super::statement::Stmt;

/// The parsed form of one module.
#[derive(Clone, Debug, PartialEq)]
pub struct SyntaxTree {
    /// The module's statements, in order, without the `__future__` imports at its start.
    pub body: Vec<Stmt>,
    /// Whether `from __future__ import annotations` defers the module's annotations, as a
    /// Python floor of 3.14 or later does for every module (LIB-29).
    pub defers_annotations: bool,
}

/// An identifier with its location.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Identifier {
    /// The text of the name.
    pub name: String,
    /// Where it appears.
    pub span: Span,
}
