//! Type aliases: `type Name = annotation` at module level makes `Name` stand for the
//! annotation wherever an annotation of the module names it (SMT-10).
//!
//! CPython evaluates an alias's value only when it is read, so the value may name a class
//! defined below it; each use becomes the value as a string annotation, which TyPy also reads
//! late. The expansion runs before the subset check, so every reader of annotations sees the
//! types themselves.

use std::collections::{BTreeMap, BTreeSet};

use typy_diagnostics::{Diagnostic, DiagnosticCode, Diagnostics, RuleId, Span};
use typy_source::syntax::{Expr, Stmt};

/// Replaces each name of a module-level type alias in the module's annotations with the
/// alias's value, and reports an alias defined twice or naming itself.
pub(super) fn expand(body: &mut [Stmt], diagnostics: &mut Diagnostics) {
    let aliases = resolved(body, diagnostics);
    for statement in body.iter_mut() {
        if matches!(statement, Stmt::TypeAlias { .. }) {
            continue;
        }
        statement_annotations(statement, &aliases);
    }
}

/// Returns each module-level alias with its value, the aliases it names already expanded.
fn resolved(body: &[Stmt], diagnostics: &mut Diagnostics) -> BTreeMap<String, Expr> {
    let mut declared: BTreeMap<String, (&Expr, Span)> = BTreeMap::new();
    for statement in body {
        let Stmt::TypeAlias {
            name,
            type_parameters,
            value,
            ..
        } = statement
        else {
            continue;
        };
        // The subset check reports type parameters.
        if !type_parameters.is_empty() {
            continue;
        }
        if let Some((_, first)) = declared.get(&name.name) {
            diagnostics.push(
                Diagnostic::new(
                    DiagnosticCode::error(414),
                    name.span,
                    format!("the type alias `{}` is defined again", name.name),
                )
                .with_rule(RuleId::SMT_10)
                .with_label(*first, "first defined here"),
            );
            continue;
        }
        declared.insert(name.name.clone(), (value, name.span));
    }
    let mut done = BTreeMap::new();
    for name in declared.keys() {
        let mut visiting = BTreeSet::new();
        resolve(name, &declared, &mut done, &mut visiting, diagnostics);
    }
    done.into_iter()
        .filter_map(|(name, value)| value.map(|value| (name, value)))
        .collect()
}

/// Expands the alias `name`, recording `None` for one that names itself through any chain of
/// aliases, which is reported once.
fn resolve(
    name: &str,
    declared: &BTreeMap<String, (&Expr, Span)>,
    done: &mut BTreeMap<String, Option<Expr>>,
    visiting: &mut BTreeSet<String>,
    diagnostics: &mut Diagnostics,
) -> Option<Expr> {
    if let Some(value) = done.get(name) {
        return value.clone();
    }
    let (value, span) = declared[name];
    if !visiting.insert(name.to_owned()) {
        diagnostics.push(
            Diagnostic::new(
                DiagnosticCode::error(201),
                span,
                format!(
                    "the type alias `{name}` names itself, and a recursive type is planned for \
                     after TyPy 1.0 (SMT-10)"
                ),
            )
            .with_rule(RuleId::SMT_10),
        );
        done.insert(name.to_owned(), None);
        return None;
    }
    let mut names = Vec::new();
    alias_names(value, declared, &mut names);
    let mut known = BTreeMap::new();
    for each in names {
        let expanded = resolve(&each, declared, done, visiting, diagnostics);
        if let Some(expanded) = expanded {
            known.insert(each, expanded);
        } else {
            done.insert(name.to_owned(), None);
            return None;
        }
    }
    let mut expanded = value.clone();
    annotation(&mut expanded, &known);
    visiting.remove(name);
    done.insert(name.to_owned(), Some(expanded.clone()));
    Some(expanded)
}

/// Collects the aliases an annotation names where an annotation may name a type.
fn alias_names(
    expression: &Expr,
    declared: &BTreeMap<String, (&Expr, Span)>,
    found: &mut Vec<String>,
) {
    match expression {
        Expr::Name { id, .. } if declared.contains_key(id) && !found.contains(id) => {
            found.push(id.clone());
        }
        Expr::Subscript { index, .. } => alias_names(index, declared, found),
        Expr::Tuple { elements, .. } | Expr::List { elements, .. } => {
            for element in elements {
                alias_names(element, declared, found);
            }
        }
        Expr::BinOp { left, right, .. } => {
            alias_names(left, declared, found);
            alias_names(right, declared, found);
        }
        Expr::Quoted { annotation, .. } => alias_names(annotation, declared, found),
        _ => {}
    }
}

/// Replaces each alias an annotation names with its value, read late as a string annotation
/// is. The name before a subscript stays, since an alias without type parameters takes none.
fn annotation(expression: &mut Expr, aliases: &BTreeMap<String, Expr>) {
    match expression {
        Expr::Name { id, span } => {
            if let Some(value) = aliases.get(id) {
                *expression = Expr::Quoted {
                    annotation: Box::new(value.clone()),
                    span: *span,
                };
            }
        }
        Expr::Subscript { index, .. } => annotation(index, aliases),
        Expr::Tuple { elements, .. } | Expr::List { elements, .. } => {
            for element in elements {
                annotation(element, aliases);
            }
        }
        Expr::BinOp { left, right, .. } => {
            annotation(left, aliases);
            annotation(right, aliases);
        }
        Expr::Quoted {
            annotation: inner, ..
        } => annotation(inner, aliases),
        _ => {}
    }
}

/// Expands the annotations of a statement and of the statements inside it.
fn statement_annotations(statement: &mut Stmt, aliases: &BTreeMap<String, Expr>) {
    match statement {
        Stmt::FunctionDef {
            parameters,
            returns,
            body,
            ..
        } => {
            for parameter in parameters {
                if let Some(each) = &mut parameter.annotation {
                    annotation(each, aliases);
                }
            }
            if let Some(each) = returns {
                annotation(each, aliases);
            }
            block(body, aliases);
        }
        Stmt::ClassDef { body, .. } | Stmt::With { body, .. } => block(body, aliases),
        Stmt::AnnAssign {
            annotation: each, ..
        } => annotation(each, aliases),
        Stmt::For { body, orelse, .. }
        | Stmt::While { body, orelse, .. }
        | Stmt::If { body, orelse, .. } => {
            block(body, aliases);
            block(orelse, aliases);
        }
        Stmt::Try {
            body,
            handlers,
            orelse,
            finalbody,
            ..
        } => {
            block(body, aliases);
            for handler in handlers {
                block(&mut handler.body, aliases);
            }
            block(orelse, aliases);
            block(finalbody, aliases);
        }
        _ => {}
    }
}

fn block(body: &mut [Stmt], aliases: &BTreeMap<String, Expr>) {
    for statement in body {
        statement_annotations(statement, aliases);
    }
}
