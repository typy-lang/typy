//! A module-level statement may only reach functions whose `def` has already run (TYP-12).

use std::collections::{BTreeMap, BTreeSet};

use typy_diagnostics::{Diagnostic, DiagnosticCode, Diagnostics, RuleId};
use typy_source::syntax::{Expr, Stmt};

use super::definition::Definition;
use super::visit::visit_statement;

/// Collects the module functions a statement list names, outside nested definitions.
fn referenced_functions(
    statements: &[Stmt],
    functions: &BTreeSet<String>,
    found: &mut BTreeSet<String>,
) {
    for statement in statements {
        visit_statement(statement, &mut |expression| {
            if let Expr::Name { id, .. } = expression
                && functions.contains(id)
            {
                found.insert(id.clone());
            }
        });
    }
}

/// A module-level statement may only reach functions whose `def` has already run; otherwise CPython raises `NameError` (TYP-12).
pub(super) fn check_calls_follow_definitions(
    statements: &[Stmt],
    functions: &[Definition<'_>],
    diagnostics: &mut Diagnostics,
) {
    let names: BTreeSet<String> = functions
        .iter()
        .map(|definition| definition.name.clone())
        .collect();
    let mut calls: BTreeMap<&str, BTreeSet<String>> = BTreeMap::new();
    for definition in functions {
        let mut found = BTreeSet::new();
        referenced_functions(definition.body, &names, &mut found);
        calls.insert(definition.name.as_str(), found);
    }
    let mut defined: BTreeSet<&str> = BTreeSet::new();
    for statement in statements {
        if let Stmt::FunctionDef { name, .. } = statement {
            defined.insert(name.name.as_str());
            continue;
        }
        let mut reached = BTreeSet::new();
        referenced_functions(std::slice::from_ref(statement), &names, &mut reached);
        let mut pending: Vec<String> = reached.iter().cloned().collect();
        while let Some(next) = pending.pop() {
            for callee in calls.get(next.as_str()).into_iter().flatten() {
                if reached.insert(callee.clone()) {
                    pending.push(callee.clone());
                }
            }
        }
        for name in reached
            .iter()
            .filter(|name| !defined.contains(name.as_str()))
        {
            diagnostics.push(
                Diagnostic::new(
                    DiagnosticCode::error(306),
                    statement.span(),
                    format!("this statement can reach `{name}` before its `def` statement runs"),
                )
                .with_rule(RuleId::TYP_12)
                .with_help(format!(
                    "move the definition of `{name}` above this statement"
                )),
            );
        }
    }
}
