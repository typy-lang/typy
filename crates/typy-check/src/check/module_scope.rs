//! The module's variables, and the rule that a name is one of a function, a class, a variable
//! or a type alias.

use std::collections::BTreeMap;

use typy_diagnostics::{Diagnostic, DiagnosticCode, Diagnostics, RuleId, Span};
use typy_source::syntax::{Expr, Identifier, Stmt};

use super::definition::Definition;
use crate::typed::ClassId;

/// Names bound by module-level statements outside function definitions.
pub(super) fn module_variables(statements: &[Stmt], names: &mut BTreeMap<String, Span>) {
    fn bind(names: &mut BTreeMap<String, Span>, target: &Expr) {
        match target {
            Expr::Name { id, span } => {
                names.entry(id.clone()).or_insert(*span);
            }
            Expr::Tuple { elements, .. } | Expr::List { elements, .. } => {
                for element in elements {
                    bind(names, element);
                }
            }
            _ => {}
        }
    }
    for statement in statements {
        // An assignment expression binds its name in the module, also inside a comprehension
        // (EXP-13).
        super::visit::visit_statement(statement, &mut |expression| {
            if let Expr::Named { target, .. } = expression {
                names.entry(target.name.clone()).or_insert(target.span);
            }
        });
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
            // The names a pattern binds are module variables, as an assignment's are (CST-06).
            Stmt::Match { cases, .. } => {
                for case in cases {
                    for name in case.pattern.bound_names() {
                        names.entry(name.name.clone()).or_insert(name.span);
                    }
                    module_variables(&case.body, names);
                }
            }
            _ => {}
        }
    }
}

/// Returns the names a function's body declares `global`, at any depth of its blocks (SMT-08).
pub(crate) fn declared_globals(statements: &[Stmt]) -> Vec<&Identifier> {
    declared(statements, &|statement| match statement {
        Stmt::Global { names, .. } => Some(names),
        _ => None,
    })
}

/// Returns the names a function's body declares `nonlocal`, at any depth of its blocks
/// (SMT-09).
pub(crate) fn declared_nonlocals(statements: &[Stmt]) -> Vec<&Identifier> {
    declared(statements, &|statement| match statement {
        Stmt::Nonlocal { names, .. } => Some(names),
        _ => None,
    })
}

/// Returns the names the declarations `pick` finds declare, at any depth of the blocks of
/// `statements`, outside nested functions and classes.
fn declared<'tree>(
    statements: &'tree [Stmt],
    pick: &dyn Fn(&'tree Stmt) -> Option<&'tree Vec<Identifier>>,
) -> Vec<&'tree Identifier> {
    let mut names = Vec::new();
    for statement in statements {
        if let Some(declared) = pick(statement) {
            names.extend(declared);
            continue;
        }
        match statement {
            Stmt::For { body, orelse, .. }
            | Stmt::While { body, orelse, .. }
            | Stmt::If { body, orelse, .. } => {
                names.extend(declared(body, pick));
                names.extend(declared(orelse, pick));
            }
            Stmt::With { body, .. } => names.extend(declared(body, pick)),
            Stmt::Match { cases, .. } => {
                for case in cases {
                    names.extend(declared(&case.body, pick));
                }
            }
            Stmt::Try { .. } => {
                for block in statement.try_blocks() {
                    names.extend(declared(block, pick));
                }
            }
            _ => {}
        }
    }
    names
}

/// A name may be a module-level function, a class or a module-level variable, only one of
/// them: CPython would rebind it.
fn check_names_are_functions_or_variables(
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

/// Reports a type alias whose name the module also gives a function, a class or a variable,
/// since an annotation reads one meaning for the name (SMT-10).
fn check_type_aliases_are_alone(
    body: &[Stmt],
    functions: &[Definition<'_>],
    classes: &BTreeMap<String, ClassId>,
    variables: &BTreeMap<String, Span>,
    diagnostics: &mut Diagnostics,
) {
    for statement in body {
        let Stmt::TypeAlias { name, .. } = statement else {
            continue;
        };
        let what = if functions
            .iter()
            .any(|definition| definition.name == name.name)
        {
            "a function"
        } else if classes.contains_key(&name.name) {
            "a class"
        } else if variables.contains_key(&name.name) {
            "a variable"
        } else {
            continue;
        };
        diagnostics.push(
            Diagnostic::new(
                DiagnosticCode::error(202),
                name.span,
                format!("`{}` is a type alias and is also {what}", name.name),
            )
            .with_rule(RuleId::SMT_10),
        );
    }
}

/// Reports a module-level name given two meanings: a class, a function, a variable or a type
/// alias.
pub(super) fn check_module_names(
    body: &[Stmt],
    functions: &[Definition<'_>],
    classes: &BTreeMap<String, ClassId>,
    variables: &BTreeMap<String, Span>,
    diagnostics: &mut Diagnostics,
) {
    check_names_are_functions_or_variables(functions, classes, variables, diagnostics);
    check_type_aliases_are_alone(body, functions, classes, variables, diagnostics);
}
