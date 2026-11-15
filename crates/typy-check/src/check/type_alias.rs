//! Type aliases: `type Name = annotation` at module level makes `Name` stand for the
//! annotation wherever an annotation of the module names it, and `type Name[T] = annotation`
//! makes `Name[int]` stand for the annotation with `int` in place of `T` (SMT-10, GEN-01).
//!
//! CPython evaluates an alias's value only when it is read, so the value may name a class
//! defined below it; each use becomes the value as a string annotation, which TyPy also reads
//! late. The expansion runs before the subset check, so every reader of annotations sees the
//! types themselves.

use std::collections::{BTreeMap, BTreeSet};

use typy_diagnostics::{Diagnostic, DiagnosticCode, Diagnostics, RuleId, Span};
use typy_source::syntax::{Expr, Stmt};

/// A type alias: its type parameters, by name, and its value.
#[derive(Clone, Debug)]
pub(crate) struct Alias {
    pub(crate) parameters: Vec<String>,
    pub(crate) value: Expr,
}

/// What expanding an annotation needs: the aliases of each scope, outermost first, the type
/// parameters in scope, which hide aliases of their names, and where to report a misused
/// alias.
struct Expansion<'diagnostics> {
    scopes: Vec<BTreeMap<String, Alias>>,
    hidden: Vec<String>,
    diagnostics: &'diagnostics mut Diagnostics,
}

/// Replaces each name of a type alias in the module's annotations with the alias's value,
/// reports an alias defined twice or naming itself, and returns the module-level aliases,
/// which other modules import (SMT-12).
pub(super) fn expand(body: &mut [Stmt], diagnostics: &mut Diagnostics) -> BTreeMap<String, Alias> {
    let aliases = resolved(body, diagnostics);
    let mut expansion = Expansion {
        scopes: vec![aliases],
        hidden: Vec::new(),
        diagnostics,
    };
    for statement in body.iter_mut() {
        if matches!(statement, Stmt::TypeAlias { .. }) {
            continue;
        }
        expansion.statement(statement);
    }
    expansion.scopes.swap_remove(0)
}

/// Returns each module-level alias with its value, the aliases it names already expanded.
fn resolved(body: &[Stmt], diagnostics: &mut Diagnostics) -> BTreeMap<String, Alias> {
    let mut declared: BTreeMap<String, Declared<'_>> = BTreeMap::new();
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
        if let Some(Declared { span: first, .. }) = declared.get(&name.name) {
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
        declared.insert(
            name.name.clone(),
            Declared {
                parameters: type_parameters
                    .iter()
                    .map(|parameter| parameter.name.name.clone())
                    .collect(),
                value,
                span: name.span,
            },
        );
    }
    let mut done = BTreeMap::new();
    for name in declared.keys() {
        let mut visiting = BTreeSet::new();
        resolve(name, &declared, &mut done, &mut visiting, diagnostics);
    }
    done.into_iter()
        .filter_map(|(name, alias)| alias.map(|alias| (name, alias)))
        .collect()
}

/// A type alias as the module declares it, before the aliases it names are expanded.
struct Declared<'tree> {
    parameters: Vec<String>,
    value: &'tree Expr,
    span: Span,
}

/// Expands the alias `name`, recording `None` for one that names itself through any chain of
/// aliases, which is reported once.
fn resolve(
    name: &str,
    declared: &BTreeMap<String, Declared<'_>>,
    done: &mut BTreeMap<String, Option<Alias>>,
    visiting: &mut BTreeSet<String>,
    diagnostics: &mut Diagnostics,
) -> Option<Alias> {
    if let Some(alias) = done.get(name) {
        return alias.clone();
    }
    let Declared {
        parameters,
        value,
        span,
    } = &declared[name];
    let span = *span;
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
    // A type parameter hides an alias of its name inside the alias's value.
    names.retain(|each| !parameters.contains(each));
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
    let mut expanded = (*value).clone();
    Expansion {
        scopes: vec![known],
        hidden: Vec::new(),
        diagnostics,
    }
    .annotation(&mut expanded);
    visiting.remove(name);
    let alias = Alias {
        parameters: parameters.clone(),
        value: expanded,
    };
    done.insert(name.to_owned(), Some(alias.clone()));
    Some(alias)
}

/// Collects the aliases an annotation names where an annotation may name a type.
fn alias_names(
    expression: &Expr,
    declared: &BTreeMap<String, Declared<'_>>,
    found: &mut Vec<String>,
) {
    match expression {
        Expr::Name { id, .. } if declared.contains_key(id) && !found.contains(id) => {
            found.push(id.clone());
        }
        Expr::Subscript { value, index, .. } => {
            alias_names(value, declared, found);
            alias_names(index, declared, found);
        }
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

impl Expansion<'_> {
    /// Returns the alias `id` names where no type parameter hides it, the innermost first.
    fn lookup(&self, id: &str) -> Option<&Alias> {
        if self.hidden.iter().any(|hidden| hidden == id) {
            return None;
        }
        self.scopes.iter().rev().find_map(|scope| scope.get(id))
    }

    /// Replaces each alias an annotation names with its value, read late as a string
    /// annotation is; a generic alias takes its type arguments in place of its parameters.
    fn annotation(&mut self, expression: &mut Expr) {
        match expression {
            Expr::Name { id, span } => {
                let Some(alias) = self.lookup(id) else {
                    return;
                };
                if !alias.parameters.is_empty() {
                    let alias = alias.clone();
                    let id = id.clone();
                    self.unapplied(&id, &alias, *span);
                    return;
                }
                *expression = Expr::Quoted {
                    annotation: Box::new(alias.value.clone()),
                    span: *span,
                };
            }
            Expr::Subscript { value, index, span } => {
                self.annotation(index);
                let generic = match value.as_ref() {
                    Expr::Name { id, .. } => self
                        .lookup(id)
                        .filter(|alias| !alias.parameters.is_empty())
                        .map(|alias| (id.clone(), alias.clone())),
                    _ => None,
                };
                let Some((name, alias)) = generic else {
                    return;
                };
                let arguments = match index.as_ref() {
                    Expr::Tuple { elements, .. } => elements.clone(),
                    one => vec![one.clone()],
                };
                if arguments.len() != alias.parameters.len() {
                    self.unapplied(&name, &alias, *span);
                    return;
                }
                let mut applied = alias.value.clone();
                substitute(&mut applied, &alias.parameters, &arguments);
                *expression = Expr::Quoted {
                    annotation: Box::new(applied),
                    span: *span,
                };
            }
            Expr::Tuple { elements, .. } | Expr::List { elements, .. } => {
                for element in elements {
                    self.annotation(element);
                }
            }
            Expr::BinOp { left, right, .. } => {
                self.annotation(left);
                self.annotation(right);
            }
            Expr::Quoted {
                annotation: inner, ..
            } => self.annotation(inner),
            _ => {}
        }
    }

    /// Reports a generic alias named without one type argument for each of its parameters.
    fn unapplied(&mut self, name: &str, alias: &Alias, span: Span) {
        self.diagnostics.push(
            Diagnostic::new(
                DiagnosticCode::error(408),
                span,
                format!(
                    "the type alias `{name}` takes {} type {}, such as `{name}[{}]`",
                    alias.parameters.len(),
                    if alias.parameters.len() == 1 {
                        "argument"
                    } else {
                        "arguments"
                    },
                    vec!["int"; alias.parameters.len()].join(", ")
                ),
            )
            .with_rule(RuleId::SMT_10),
        );
    }

    /// Expands the annotations of a statement and of the statements inside it.
    fn statement(&mut self, statement: &mut Stmt) {
        match statement {
            Stmt::FunctionDef {
                parameters,
                returns,
                body,
                type_parameters,
                ..
            } => {
                let outer = self.hide(type_parameters);
                for parameter in parameters {
                    if let Some(each) = &mut parameter.annotation {
                        self.annotation(each);
                    }
                }
                if let Some(each) = returns {
                    self.annotation(each);
                }
                self.block(body);
                self.hidden.truncate(outer);
            }
            Stmt::ClassDef {
                body,
                type_parameters,
                ..
            } => {
                let outer = self.hide(type_parameters);
                self.block(body);
                self.hidden.truncate(outer);
            }
            Stmt::With { body, .. } => self.block(body),
            Stmt::Match { cases, .. } => {
                for case in cases {
                    self.block(&mut case.body);
                }
            }
            Stmt::AnnAssign {
                annotation: each, ..
            } => self.annotation(each),
            Stmt::For { body, orelse, .. }
            | Stmt::While { body, orelse, .. }
            | Stmt::If { body, orelse, .. } => {
                self.block(body);
                self.block(orelse);
            }
            Stmt::Try {
                body,
                handlers,
                orelse,
                finalbody,
                ..
            } => {
                self.block(body);
                for handler in handlers {
                    self.block(&mut handler.body);
                }
                self.block(orelse);
                self.block(finalbody);
            }
            _ => {}
        }
    }

    /// Puts a definition's type parameters in scope; returns how many were in scope before.
    fn hide(&mut self, parameters: &[typy_source::syntax::TypeParameter]) -> usize {
        let outer = self.hidden.len();
        self.hidden.extend(
            parameters
                .iter()
                .map(|parameter| parameter.name.name.clone()),
        );
        outer
    }

    /// Expands a block: an alias it defines applies to the statements after it and the
    /// blocks inside them, as the name it binds does (SMT-12).
    fn block(&mut self, body: &mut [Stmt]) {
        self.scopes.push(BTreeMap::new());
        for statement in body {
            if let Stmt::TypeAlias {
                name,
                type_parameters,
                value,
                ..
            } = statement
            {
                self.local_alias(name, type_parameters, value);
            } else {
                self.statement(statement);
            }
        }
        self.scopes.pop();
    }

    /// Adds an alias a block defines to the block's scope, its value expanded in the scopes
    /// around it.
    fn local_alias(
        &mut self,
        name: &typy_source::syntax::Identifier,
        type_parameters: &[typy_source::syntax::TypeParameter],
        value: &Expr,
    ) {
        let parameters: Vec<String> = type_parameters
            .iter()
            .map(|parameter| parameter.name.name.clone())
            .collect();
        let mut names = Vec::new();
        let mut own = BTreeMap::new();
        own.insert(
            name.name.clone(),
            Declared {
                parameters: parameters.clone(),
                value,
                span: name.span,
            },
        );
        alias_names(value, &own, &mut names);
        if !names.is_empty() && !parameters.contains(&name.name) {
            self.diagnostics.push(
                Diagnostic::new(
                    DiagnosticCode::error(201),
                    name.span,
                    format!(
                        "the type alias `{}` names itself, and a recursive type is planned \
                         for after TyPy 1.0 (SMT-10)",
                        name.name
                    ),
                )
                .with_rule(RuleId::SMT_10),
            );
            return;
        }
        let mut expanded = value.clone();
        let outer = self.hidden.len();
        self.hidden.extend(parameters.iter().cloned());
        self.annotation(&mut expanded);
        self.hidden.truncate(outer);
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(
                name.name.clone(),
                Alias {
                    parameters,
                    value: expanded,
                },
            );
        }
    }
}

/// Replaces each type parameter an alias's value names with its type argument.
fn substitute(expression: &mut Expr, parameters: &[String], arguments: &[Expr]) {
    match expression {
        Expr::Name { id, .. } => {
            if let Some(index) = parameters.iter().position(|parameter| parameter == id) {
                *expression = arguments[index].clone();
            }
        }
        Expr::Subscript { value, index, .. } => {
            substitute(value, parameters, arguments);
            substitute(index, parameters, arguments);
        }
        Expr::Tuple { elements, .. } | Expr::List { elements, .. } => {
            for element in elements {
                substitute(element, parameters, arguments);
            }
        }
        Expr::BinOp { left, right, .. } => {
            substitute(left, parameters, arguments);
            substitute(right, parameters, arguments);
        }
        Expr::Quoted { annotation, .. } => substitute(annotation, parameters, arguments),
        _ => {}
    }
}
