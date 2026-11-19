//! CPython's compile-time rules for `global` and `nonlocal` in a function, which the parser
//! accepts and CPython's compiler rejects as a `SyntaxError` (SMT-08, SMT-09, STD-01).

use std::collections::BTreeSet;

use typy_diagnostics::Span;
use typy_source::syntax::Identifier;

use super::Walker;

/// The names a function's body has used, assigned, annotated or declared global or nonlocal
/// so far, in the order of its source, and whether a function encloses it.
#[derive(Default)]
pub(super) struct GlobalScope {
    parameters: BTreeSet<String>,
    used: BTreeSet<String>,
    assigned: BTreeSet<String>,
    annotated: BTreeSet<String>,
    globals: BTreeSet<String>,
    nonlocals: BTreeSet<String>,
    nested: bool,
}

impl GlobalScope {
    /// Returns the scope of a function with these parameters, `nested` in another function
    /// or not.
    pub(super) fn of_parameters<'name>(
        names: impl Iterator<Item = &'name str>,
        nested: bool,
    ) -> Self {
        Self {
            parameters: names.map(str::to_owned).collect(),
            nested,
            ..Self::default()
        }
    }
}

impl Walker {
    /// Notes a name the function reads.
    pub(super) fn used_name(&mut self, id: &str) {
        if let Some(scope) = &mut self.scope {
            scope.used.insert(id.to_owned());
        }
    }

    /// Notes a name the function assigns.
    pub(super) fn assigned_name(&mut self, id: &str) {
        if let Some(scope) = &mut self.scope {
            scope.assigned.insert(id.to_owned());
        }
    }

    /// Notes a name the function annotates, which may not also be global.
    pub(super) fn annotated_name(&mut self, id: &str, span: Span) {
        let Some(scope) = &mut self.scope else {
            return;
        };
        scope.annotated.insert(id.to_owned());
        if scope.globals.contains(id) {
            self.syntax_error(span, &format!("annotated name '{id}' can't be global"));
        } else if scope.nonlocals.contains(id) {
            self.syntax_error(span, &format!("annotated name '{id}' can't be nonlocal"));
        }
    }

    /// Checks `global names` in a function against what came before it, as CPython does.
    pub(super) fn global_statement(&mut self, names: &[Identifier]) {
        let Some(scope) = &mut self.scope else {
            return;
        };
        let mut errors = Vec::new();
        for name in names {
            let id = name.name.as_str();
            let message = if scope.parameters.contains(id) {
                Some(format!("name '{id}' is parameter and global"))
            } else if scope.nonlocals.contains(id) {
                Some(format!("name '{id}' is nonlocal and global"))
            } else if scope.annotated.contains(id) {
                Some(format!("annotated name '{id}' can't be global"))
            } else if scope.assigned.contains(id) {
                Some(format!(
                    "name '{id}' is assigned to before global declaration"
                ))
            } else if scope.used.contains(id) {
                Some(format!("name '{id}' is used prior to global declaration"))
            } else {
                None
            };
            scope.globals.insert(id.to_owned());
            if let Some(message) = message {
                errors.push((name.span, message));
            }
        }
        for (span, message) in errors {
            self.syntax_error(span, &message);
        }
    }

    /// Checks `nonlocal names` against where it stands and what came before it, as CPython
    /// does; whether a function around it binds each name is checked with the function's
    /// variables.
    pub(super) fn nonlocal_statement(&mut self, names: &[Identifier], span: Span) {
        let Some(scope) = &mut self.scope else {
            self.syntax_error(span, "nonlocal declaration not allowed at module level");
            return;
        };
        let mut errors = Vec::new();
        for name in names {
            let id = name.name.as_str();
            let message = if scope.parameters.contains(id) {
                Some(format!("name '{id}' is parameter and nonlocal"))
            } else if scope.globals.contains(id) {
                Some(format!("name '{id}' is nonlocal and global"))
            } else if scope.annotated.contains(id) {
                Some(format!("annotated name '{id}' can't be nonlocal"))
            } else if scope.assigned.contains(id) {
                Some(format!(
                    "name '{id}' is assigned to before nonlocal declaration"
                ))
            } else if scope.used.contains(id) {
                Some(format!("name '{id}' is used prior to nonlocal declaration"))
            } else if !scope.nested {
                Some(format!("no binding for nonlocal '{id}' found"))
            } else {
                None
            };
            scope.nonlocals.insert(id.to_owned());
            if let Some(message) = message {
                errors.push((name.span, message));
            }
        }
        for (span, message) in errors {
            self.syntax_error(span, &message);
        }
    }

    /// Checks where a lazy import stands, by CPython 3.15's rules: only outside functions,
    /// classes and `try` statements, and never of every name or from `__future__` (MOD-06).
    pub(super) fn lazy_import(
        &mut self,
        statement: &typy_source::syntax::Stmt,
        span: Span,
        place: super::Place,
    ) {
        use typy_source::syntax::Stmt;
        let message = match (place, statement) {
            (super::Place::Function, _) => Some("lazy import not allowed inside functions"),
            (super::Place::Class, _) => Some("lazy import not allowed inside classes"),
            _ if self.try_depth > 0 => Some("lazy import not allowed inside try/except blocks"),
            (_, Stmt::ImportFrom { names, .. })
                if names.iter().any(|alias| alias.name.name == "*") =>
            {
                Some("lazy from ... import * is not allowed")
            }
            (
                _,
                Stmt::ImportFrom {
                    module: Some(module),
                    level: 0,
                    ..
                },
            ) if module.name == "__future__" => Some("lazy from __future__ import is not allowed"),
            _ => None,
        };
        if let Some(message) = message {
            self.syntax_error(span, message);
        }
    }
}
