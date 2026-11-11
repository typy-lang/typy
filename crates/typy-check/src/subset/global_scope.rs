//! CPython's compile-time rules for `global` in a function, which the parser accepts and
//! CPython's compiler rejects as a `SyntaxError` (SMT-08, STD-01).

use std::collections::BTreeSet;

use typy_diagnostics::Span;
use typy_source::syntax::Identifier;

use super::Walker;

/// The names a function's body has used, assigned, annotated or declared global so far, in
/// the order of its source.
#[derive(Default)]
pub(super) struct GlobalScope {
    parameters: BTreeSet<String>,
    used: BTreeSet<String>,
    assigned: BTreeSet<String>,
    annotated: BTreeSet<String>,
    globals: BTreeSet<String>,
}

impl GlobalScope {
    /// Returns the scope of a function with these parameters.
    pub(super) fn of_parameters<'name>(names: impl Iterator<Item = &'name str>) -> Self {
        Self {
            parameters: names.map(str::to_owned).collect(),
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
}
