//! The module's variables, and the rule that a name is a function or a variable, not both.

use std::collections::BTreeMap;

use typy_diagnostics::{Diagnostic, DiagnosticCode, Diagnostics, RuleId, Span};
use typy_source::syntax::{Expr, Stmt};

use super::definition::Definition;
use crate::typed::ClassId;

/// Names bound by module-level statements outside function definitions.
pub(super) fn module_variables(statements: &[Stmt], names: &mut BTreeMap<String, Span>) {
    fn bind(names: &mut BTreeMap<String, Span>, target: &Expr) {
        match target {
            Expr::Name { id, span } => {
                names.entry(id.clone()).or_insert(*span);
            }
            Expr::Tuple { elements, .. } => {
                for element in elements {
                    bind(names, element);
                }
            }
            _ => {}
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
            Stmt::Try { .. } => {
                for block in statement.try_blocks() {
                    module_variables(block, names);
                }
            }
            Stmt::With { items, body, .. } => {
                for target in items.iter().filter_map(|item| item.target.as_ref()) {
                    bind(names, target);
                }
                module_variables(body, names);
            }
            _ => {}
        }
    }
}

/// A name may be a module-level function, a class or a module-level variable, only one of
/// them: CPython would rebind it.
pub(super) fn check_names_are_functions_or_variables(
    functions: &[Definition<'_>],
    classes: &BTreeMap<String, ClassId>,
    variables: &BTreeMap<String, Span>,
    diagnostics: &mut Diagnostics,
) {
    for name in classes.keys() {
        let function = functions.iter().find(|definition| definition.name == *name);
        let clash = match (function, variables.get(name)) {
            (Some(function), _) => Some((function.span, "a function")),
            (None, Some(span)) => Some((*span, "assigned as a variable")),
            (None, None) => None,
        };
        if let Some((span, what)) = clash {
            diagnostics.push(
                Diagnostic::new(
                    DiagnosticCode::error(202),
                    span,
                    format!("`{name}` is a class and is also {what}"),
                )
                .with_rule(RuleId::STD_04),
            );
        }
    }
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
