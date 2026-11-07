//! The module's variables, and the rule that a name is a function or a variable, not both.

use std::collections::BTreeMap;

use typy_diagnostics::{Diagnostic, DiagnosticCode, Diagnostics, RuleId, Span};
use typy_source::syntax::{Expr, Stmt};

use super::definition::Definition;

/// Names bound by module-level statements outside function definitions.
pub(super) fn module_variables(statements: &[Stmt], names: &mut BTreeMap<String, Span>) {
    fn bind(names: &mut BTreeMap<String, Span>, target: &Expr) {
        if let Expr::Name { id, span } = target {
            names.entry(id.clone()).or_insert(*span);
        }
    }
    for statement in statements {
        match statement {
            Stmt::Assign { targets, .. } => targets.iter().for_each(|target| bind(names, target)),
            Stmt::AugAssign { target, .. } | Stmt::AnnAssign { target, .. } => bind(names, target),
            Stmt::For {
                target,
                body,
                orelse,
                ..
            } => {
                bind(names, target);
                module_variables(body, names);
                module_variables(orelse, names);
            }
            Stmt::While { body, orelse, .. } | Stmt::If { body, orelse, .. } => {
                module_variables(body, names);
                module_variables(orelse, names);
            }
            _ => {}
        }
    }
}

/// A name may be a module-level function or a module-level variable, not both: CPython would rebind it.
pub(super) fn check_names_are_functions_or_variables(
    functions: &[Definition<'_>],
    variables: &BTreeMap<String, Span>,
    diagnostics: &mut Diagnostics,
) {
    for definition in functions {
        if let Some(span) = variables.get(&definition.name) {
            diagnostics.push(
                Diagnostic::new(
                    DiagnosticCode::error(202),
                    *span,
                    format!(
                        "`{}` is a function and is also assigned as a variable",
                        definition.name
                    ),
                )
                .with_rule(RuleId::STD_04),
            );
        }
    }
}
