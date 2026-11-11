//! Which imports of a module are lazy: `lazy import` and `lazy from ... import`, and the
//! imports of the modules a module-level `__lazy_modules__` lists, as PEP 810 defines them
//! for Python 3.15 (MOD-06, MOD-07).

use std::collections::BTreeSet;

use crate::syntax::{Constant, Expr, Stmt};

/// Returns the module names a module-level `__lazy_modules__ = [...]` of string literals
/// lists, in any of its assignments at module level.
#[must_use]
pub fn lazy_modules(statements: &[Stmt]) -> BTreeSet<String> {
    let mut listed = BTreeSet::new();
    for statement in statements {
        let (targets, value) = match statement {
            Stmt::Assign { targets, value, .. } => (targets.as_slice(), value),
            Stmt::AnnAssign {
                target,
                value: Some(value),
                ..
            } => (std::slice::from_ref(target), value),
            _ => continue,
        };
        let names_list = targets
            .iter()
            .any(|target| matches!(target, Expr::Name { id, .. } if id == "__lazy_modules__"));
        if let (true, Expr::List { elements, .. } | Expr::Tuple { elements, .. }) =
            (names_list, value)
        {
            listed.extend(elements.iter().filter_map(|element| match element {
                Expr::Constant {
                    value: Constant::Str(text),
                    ..
                } => Some(text.clone()),
                _ => None,
            }));
        }
    }
    listed
}

/// Returns whether an import statement is lazy: written `lazy`, or naming only modules that
/// `__lazy_modules__` lists. A `from` import counts by the module it imports from, as given.
#[must_use]
pub fn is_lazy(statement: &Stmt, listed: &BTreeSet<String>) -> bool {
    match statement {
        Stmt::Import { is_lazy, names, .. } => {
            *is_lazy
                || (!names.is_empty()
                    && names.iter().all(|alias| listed.contains(&alias.name.name)))
        }
        Stmt::ImportFrom {
            is_lazy,
            module,
            level,
            ..
        } => {
            *is_lazy
                || (*level == 0
                    && module
                        .as_ref()
                        .is_some_and(|module| listed.contains(&module.name)))
        }
        _ => false,
    }
}
