//! Finds the names by which a module reads one name of a standard library module, such as
//! `typing.overload` or `abc.ABC`, before the checker resolves its imports: the names a `from`
//! import binds to it, and the names an `import` binds to its module.

use std::collections::BTreeSet;

use typy_source::syntax::{Expr, Stmt};

/// The names a module's imports bind to one name of a module, and to that module.
#[derive(Clone, Debug)]
pub(super) struct ImportedName {
    name: &'static str,
    direct: BTreeSet<String>,
    modules: BTreeSet<String>,
}

impl ImportedName {
    /// Returns the names the module-level imports of `body` bind to `module.name` and to
    /// `module`.
    pub(super) fn of(body: &[Stmt], module: &str, name: &'static str) -> Self {
        let mut found = Self {
            name,
            direct: BTreeSet::new(),
            modules: BTreeSet::new(),
        };
        for statement in body {
            match statement {
                Stmt::ImportFrom {
                    module: Some(from),
                    names,
                    level: 0,
                    ..
                } if from.name == module => {
                    for alias in names.iter().filter(|alias| alias.name.name == name) {
                        let bound = alias.as_name.as_ref().unwrap_or(&alias.name);
                        found.direct.insert(bound.name.clone());
                    }
                }
                Stmt::Import { names, .. } => {
                    for alias in names.iter().filter(|alias| alias.name.name == module) {
                        let bound = alias.as_name.as_ref().unwrap_or(&alias.name);
                        found.modules.insert(bound.name.clone());
                    }
                }
                _ => {}
            }
        }
        found
    }

    /// Returns whether no import binds the name or its module.
    pub(super) fn is_empty(&self) -> bool {
        self.direct.is_empty() && self.modules.is_empty()
    }

    /// Returns whether `expression` names it: a name bound to it, or the name read through
    /// its module.
    pub(super) fn names(&self, expression: &Expr) -> bool {
        match expression {
            Expr::Name { id, .. } => self.direct.contains(id),
            Expr::Attribute {
                value, attribute, ..
            } => {
                attribute.name == self.name
                    && matches!(value.as_ref(), Expr::Name { id, .. } if self.modules.contains(id))
            }
            _ => false,
        }
    }
}
