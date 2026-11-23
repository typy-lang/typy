//! Infers the result of a function of the module without a result annotation from its
//! `return` statements, each callee before its callers (TYP-26, ADR-0092).

use std::collections::{BTreeMap, BTreeSet};

use typy_diagnostics::Diagnostics;
use typy_source::syntax::{Expr, Stmt};

use super::check_body;
use super::definition::{Definition, missing_result};
use super::definition_order::referenced_functions;
use crate::body::{Program, has_yield};
use crate::typed::FunctionId;
use crate::types::Type;

/// Infers the results of the functions of `module_body`, the first `count` of `definitions`
/// from `first_function` on, that have no result annotation. A generator, a function the
/// library exports, named in `exported`, and one that calls itself, directly or through
/// others, keep the annotation, E0204. Each body is looked at with its diagnostics dropped
/// and its effects undone, and the result is the union of the types its `return` statements
/// give, `None` among them when it can reach its end. Returns whether any was inferred.
pub(super) fn infer_results(
    program: &mut Program,
    module_body: &[Stmt],
    (definitions, count, first_function, first_class): (&mut [Definition<'_>], usize, usize, usize),
    exported: &BTreeSet<String>,
    diagnostics: &mut Diagnostics,
) -> bool {
    let unannotated = unannotated_functions(module_body);
    let pending: Vec<usize> = (0..count)
        .filter(|index| unannotated.contains_key(&definitions[*index].name))
        .collect();
    if pending.is_empty() {
        return false;
    }
    let names: BTreeSet<String> = pending
        .iter()
        .map(|index| definitions[*index].name.clone())
        .collect();
    let mut calls: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for index in &pending {
        let mut found = BTreeSet::new();
        referenced_functions(definitions[*index].body, &names, &mut found);
        calls.insert(definitions[*index].name.clone(), found);
    }
    let order = callees_first(&pending, definitions, &calls);
    let mut inferred = false;
    for (index, cycle) in order {
        let definition = &definitions[index];
        let reason = if cycle {
            Some((
                "it calls itself, directly or through other functions",
                "write its result type, such as `-> int`",
            ))
        } else if definition.body.iter().any(has_yield) {
            Some((
                "a generator's type depends on what it yields and is sent",
                "write its type, such as `-> Generator[int, None, None]`",
            ))
        } else if exported.contains(&definition.name) {
            Some((
                "the library exports it, so its result is part of the library's interface",
                "write its result type, such as `-> int`",
            ))
        } else {
            None
        };
        if let Some(reason) = reason {
            let at = unannotated
                .get(&definition.name)
                .copied()
                .unwrap_or(definition.span);
            missing_result(&definition.name, at, Some(reason), diagnostics);
            continue;
        }
        let id = FunctionId(u32::try_from(first_function + index).unwrap_or(u32::MAX));
        let result = look_at(program, id, definition, first_class);
        definitions[index].signature.returns = result.clone();
        program.set_result(id, result);
        inferred = true;
    }
    inferred
}

/// Returns the result the body of `definition` gives, after looking at it with its
/// diagnostics dropped and its effects undone.
fn look_at(
    program: &mut Program,
    id: FunctionId,
    definition: &Definition<'_>,
    first_class: usize,
) -> Type {
    let snapshot = program.snapshot(first_class);
    program.inferring = Some((id, Vec::new()));
    check_body(program, id, definition, &mut Diagnostics::new());
    let returned = program
        .inferring
        .take()
        .map(|(_, types)| types)
        .unwrap_or_default();
    program.undo(snapshot);
    program.forget_function(id);
    Type::union(returned)
}

/// Returns the names of the module's functions without a result annotation, with where each
/// name stands; a module defines each name once (STD-04).
fn unannotated_functions(module_body: &[Stmt]) -> BTreeMap<String, typy_diagnostics::Span> {
    module_body
        .iter()
        .filter_map(|statement| match statement {
            Stmt::FunctionDef {
                name,
                returns: None,
                ..
            } => Some((name.name.clone(), name.span)),
            _ => None,
        })
        .collect()
}

/// Returns the pending functions, each after the functions it calls, and whether it is part
/// of a cycle of calls, which a depth-first walk finds as a function reached again while it
/// is being walked.
fn callees_first(
    pending: &[usize],
    definitions: &[Definition<'_>],
    calls: &BTreeMap<String, BTreeSet<String>>,
) -> Vec<(usize, bool)> {
    let index_of: BTreeMap<&str, usize> = pending
        .iter()
        .map(|index| (definitions[*index].name.as_str(), *index))
        .collect();
    let mut order: Vec<(usize, bool)> = Vec::new();
    let mut done: BTreeSet<usize> = BTreeSet::new();
    let mut cyclic: BTreeSet<usize> = BTreeSet::new();
    for start in pending {
        let mut path: Vec<usize> = Vec::new();
        visit(
            *start,
            &index_of,
            (definitions, calls),
            (&mut path, &mut done, &mut cyclic),
            &mut order,
        );
    }
    order
        .into_iter()
        .map(|(index, _)| (index, cyclic.contains(&index)))
        .collect()
}

/// Walks the calls of the function at `index`, appending each function after its callees.
fn visit(
    index: usize,
    index_of: &BTreeMap<&str, usize>,
    (definitions, calls): (&[Definition<'_>], &BTreeMap<String, BTreeSet<String>>),
    (path, done, cyclic): (&mut Vec<usize>, &mut BTreeSet<usize>, &mut BTreeSet<usize>),
    order: &mut Vec<(usize, bool)>,
) {
    if let Some(position) = path.iter().position(|each| *each == index) {
        cyclic.extend(path[position..].iter().copied());
        return;
    }
    if done.contains(&index) {
        return;
    }
    path.push(index);
    let called = calls
        .get(&definitions[index].name)
        .into_iter()
        .flatten()
        .filter_map(|name| index_of.get(name.as_str()).copied());
    for callee in called.collect::<Vec<_>>() {
        visit(
            callee,
            index_of,
            (definitions, calls),
            (&mut *path, &mut *done, &mut *cyclic),
            order,
        );
    }
    path.pop();
    done.insert(index);
    order.push((index, false));
}

/// Returns the names `__all__` lists in `module_body`, the functions a library exports.
pub(super) fn exported_names(module_body: &[Stmt]) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    for statement in module_body {
        let (targets, value) = match statement {
            Stmt::Assign { targets, value, .. } => (targets.as_slice(), value),
            _ => continue,
        };
        let names_all = |target: &Expr| matches!(target, Expr::Name { id, .. } if id == "__all__");
        if !targets.iter().any(names_all) {
            continue;
        }
        if let Expr::List { elements, .. } | Expr::Tuple { elements, .. } = value {
            for element in elements {
                if let Expr::Constant {
                    value: typy_source::syntax::Constant::Str(text),
                    ..
                } = element
                {
                    names.insert(text.clone());
                }
            }
        }
    }
    names
}
