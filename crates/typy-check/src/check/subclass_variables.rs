//! Finds, before any module is checked, the class variables a program assigns through a
//! subclass that only inherits them (CLS-25, ADR-0037). In CPython, `Square.sides = 4` gives
//! `Square` a variable of its own, which its instances and subclasses then read; a read through
//! an instance of the base may so read the variable of the instance's class, even in a module
//! checked before the one that assigns it.

use std::collections::BTreeSet;

use typy_source::Module;
use typy_source::syntax::{Expr, Stmt};

use super::class_index::ClassIndex;

/// A class of the program by its module and its name, and the name of a variable.
pub(crate) type ClassVariable = (String, String, String);

/// The class variables of subclasses a program assigns.
#[derive(Clone, Debug, Default)]
pub(crate) struct SubclassVariables {
    /// Each class an assignment through it gives a variable of its own, with the variable.
    pub(crate) own: BTreeSet<ClassVariable>,
    /// Each class whose body gives a variable that some class derived from it gets one of its
    /// own of: a read through an instance of it, or of a class derived from it, reads the
    /// variable of the instance's class.
    pub(crate) shared: BTreeSet<ClassVariable>,
}

/// Returns the class variables `modules` assign through a subclass that inherits them.
pub(crate) fn subclass_variables(modules: &[Module]) -> SubclassVariables {
    let index = ClassIndex::of(modules);
    let mut found = SubclassVariables::default();
    for module in modules {
        let name = module.name.as_str();
        let mut assigned = Vec::new();
        assignments(&module.syntax.body, &mut assigned);
        for (target, variable) in assigned {
            let Some(class) = index.class_of(name, target) else {
                continue;
            };
            if let Some(holder) = holder(&index, &class, variable) {
                if holder == class {
                    continue;
                }
                found.own.insert((class.0, class.1, variable.to_owned()));
                found
                    .shared
                    .insert((holder.0, holder.1, variable.to_owned()));
            }
        }
    }
    found
}

/// Returns the class whose body gives `variable`: `class` itself or the nearest base.
fn holder(
    index: &ClassIndex<'_>,
    class: &(String, String),
    variable: &str,
) -> Option<(String, String)> {
    index.chain(class).into_iter().find(|each| {
        index
            .body(each)
            .iter()
            .filter_map(given_name)
            .any(|name| name == variable)
    })
}

/// Returns the name a statement of a class body gives a value: a class variable or the
/// default of an instance attribute.
fn given_name(statement: &Stmt) -> Option<String> {
    match statement {
        Stmt::Assign { targets, .. } => match targets.as_slice() {
            [Expr::Name { id, .. }] => Some(id.clone()),
            _ => None,
        },
        Stmt::AnnAssign {
            target: Expr::Name { id, .. },
            value: Some(_),
            ..
        } => Some(id.clone()),
        _ => None,
    }
}

/// Collects each `object.name = value` and `object.name op= value` of `statements`, at any
/// depth, with its object and name.
fn assignments<'tree>(statements: &'tree [Stmt], found: &mut Vec<(&'tree Expr, &'tree str)>) {
    for statement in statements {
        let targets: Vec<&Expr> = match statement {
            Stmt::Assign { targets, .. } => targets.iter().collect(),
            Stmt::AugAssign { target, .. } => vec![target],
            _ => Vec::new(),
        };
        for target in targets {
            if let Expr::Attribute {
                value, attribute, ..
            } = target
            {
                found.push((value.as_ref(), attribute.name.as_str()));
            }
        }
        for block in blocks(statement) {
            assignments(block, found);
        }
    }
}

/// Returns the blocks a statement holds: the bodies of a `def`, a `class`, a loop, an `if`,
/// a `with`, a `try` and the cases of a `match`.
fn blocks(statement: &Stmt) -> Vec<&[Stmt]> {
    match statement {
        Stmt::FunctionDef { body, .. } | Stmt::ClassDef { body, .. } | Stmt::With { body, .. } => {
            vec![body]
        }
        Stmt::If { body, orelse, .. }
        | Stmt::For { body, orelse, .. }
        | Stmt::While { body, orelse, .. } => vec![body, orelse],
        Stmt::Match { cases, .. } => cases.iter().map(|case| case.body.as_slice()).collect(),
        Stmt::Try { .. } => statement.try_blocks(),
        _ => Vec::new(),
    }
}
