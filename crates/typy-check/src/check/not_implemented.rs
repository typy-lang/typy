//! Finds, before any module is checked, the operator and comparison methods that may return
//! `NotImplemented` (CLS-21, ADR-0038). A method of such a name returns `T | NotImplementedType`
//! in every class of a chain of bases where one of them returns `NotImplemented`, so a call
//! through the method table finds one return type in every class.

use std::collections::BTreeSet;

use typy_diagnostics::Span;
use typy_source::Module;
use typy_source::syntax::{Expr, Stmt};

use super::class_index::ClassIndex;

/// The methods whose return type takes `NotImplemented`, each by the span of its class's
/// statement and its name.
pub(crate) type GivingWay = BTreeSet<(Span, String)>;

/// Returns the methods of the classes of `modules` that return `T | NotImplementedType`.
pub(crate) fn not_implemented_methods(modules: &[Module]) -> GivingWay {
    let index = ClassIndex::of(modules);
    let root = |key| index.chain(key).pop();
    let families: BTreeSet<_> = index
        .classes()
        .flat_map(|(key, body, _)| {
            operator_methods(body)
                .filter(|(_, method)| returns_not_implemented(method))
                .map(move |(name, _)| (root(key), name))
        })
        .collect();
    let mut found = GivingWay::new();
    for (key, body, span) in index.classes() {
        let root = root(key);
        for (name, _) in operator_methods(body) {
            if families.contains(&(root.clone(), name.clone())) {
                found.insert((span, name));
            }
        }
    }
    found
}

/// Returns true for a method an operator calls that may give way with `NotImplemented`: an
/// arithmetic, reflected or in-place one, or a comparison.
pub(crate) fn gives_way(name: &str) -> bool {
    matches!(
        name,
        "__eq__" | "__ne__" | "__lt__" | "__le__" | "__gt__" | "__ge__"
    ) || super::class::is_binary_method(name)
}

/// Returns each method of a class body that may give way, with its body.
fn operator_methods(body: &[Stmt]) -> impl Iterator<Item = (String, &[Stmt])> {
    body.iter().filter_map(|statement| match statement {
        Stmt::FunctionDef { name, body, .. } if gives_way(&name.name) => {
            Some((name.name.clone(), body.as_slice()))
        }
        _ => None,
    })
}

/// Returns true when `statements`, outside nested definitions, hold `return NotImplemented`.
fn returns_not_implemented(statements: &[Stmt]) -> bool {
    statements.iter().any(|statement| match statement {
        Stmt::Return {
            value: Some(Expr::Name { id, .. }),
            ..
        } => id == "NotImplemented",
        Stmt::If { body, orelse, .. }
        | Stmt::For { body, orelse, .. }
        | Stmt::While { body, orelse, .. } => {
            returns_not_implemented(body) || returns_not_implemented(orelse)
        }
        Stmt::With { body, .. } => returns_not_implemented(body),
        Stmt::Match { cases, .. } => cases.iter().any(|case| returns_not_implemented(&case.body)),
        Stmt::Try { .. } => statement
            .try_blocks()
            .into_iter()
            .any(returns_not_implemented),
        _ => false,
    })
}

#[cfg(test)]
mod tests;
