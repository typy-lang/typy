//! The parsed form of one module, and identifiers.

use typy_diagnostics::Span;

use super::statement::Stmt;

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
