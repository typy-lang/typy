//! The names a module's module-level imports bind, which its tables and annotations need
//! before its functions and classes are collected, and the module variables assigned through
//! module objects anywhere in the program (SMT-07, ADR-0015).

use std::collections::{BTreeMap, BTreeSet};

use typy_diagnostics::Span;
use typy_source::syntax::{Expr, Stmt};

use crate::body::{Imported, Program};
use crate::typed::{ClassId, FunctionId};

/// What a module's module-level imports bind.
#[derive(Default)]
pub(super) struct ModuleImports {
    /// The functions imported by name, which the module's table of functions takes.
    pub(super) functions: BTreeMap<String, FunctionId>,
    /// The classes imported by name, which the module's table of classes takes.
    pub(super) classes: BTreeMap<String, ClassId>,
    /// The names bound to modules or to values, which become module variables.
    pub(super) variables: BTreeMap<String, Span>,
    /// For each name bound to a module, the module.
    pub(super) modules: BTreeMap<String, String>,
    /// The names of `typing`, `collections.abc` and `types` imported for annotations.
    pub(super) annotations: BTreeMap<String, crate::check::AnnotationName>,
    /// For each name bound to one of those modules, the module.
    pub(super) standard: BTreeMap<String, String>,
}

/// Returns what the imports at module level bind, in any block outside a `def` or a `class`.
/// An import that cannot be resolved binds nothing here; the module body reports it.
pub(super) fn module_imports(program: &Program, statements: &[Stmt]) -> ModuleImports {
    let mut found = ModuleImports::default();
    collect(program, statements, &mut found);
    found
}

fn collect(program: &Program, statements: &[Stmt], found: &mut ModuleImports) {
    for statement in statements {
        match statement {
            Stmt::Import { .. } | Stmt::ImportFrom { .. } => {
                let Ok(resolved) = program.resolve_import(statement) else {
                    continue;
                };
                for (name, imported, span) in resolved.names {
                    match imported {
                        Imported::Function(function) => {
                            found.functions.insert(name, function);
                        }
                        Imported::Class(class) => {
                            found.classes.insert(name, class);
                        }
                        Imported::Module(module) => {
                            found.variables.entry(name.clone()).or_insert(span);
                            found.modules.insert(name, module.as_str().to_owned());
                        }
                        Imported::Variable { .. } => {
                            found.variables.entry(name).or_insert(span);
                        }
                        Imported::Annotation(annotation) => {
                            found.annotations.insert(name, annotation);
                        }
                        Imported::Standard(module) => {
                            found.standard.insert(name, module);
                        }
                    }
                }
            }
            Stmt::If { body, orelse, .. }
            | Stmt::For { body, orelse, .. }
            | Stmt::While { body, orelse, .. } => {
                collect(program, body, found);
                collect(program, orelse, found);
            }
            Stmt::With { body, .. } => collect(program, body, found),
            Stmt::Try { .. } => {
                for block in statement.try_blocks() {
                    collect(program, block, found);
                }
            }
            _ => {}
        }
    }
}

/// Returns each `(module, name)` that a statement of `statements`, at any depth, assigns
/// through a name `import` bound to the module, such as `settings.debug = True`.
pub(super) fn assigned_through_modules(statements: &[Stmt]) -> BTreeSet<(String, String)> {
    let mut aliases = BTreeMap::new();
    find_aliases(statements, &mut aliases);
    let mut assigned = BTreeSet::new();
    find_assignments(statements, &aliases, &mut assigned);
    assigned
}

fn find_aliases(statements: &[Stmt], aliases: &mut BTreeMap<String, String>) {
    for statement in statements {
        match statement {
            Stmt::Import { names, .. } => {
                for alias in names {
                    let module = alias.name.name.clone();
                    let bound = alias.as_name.as_ref().map_or_else(
                        || module.split('.').next().unwrap_or_default().to_owned(),
                        |as_name| as_name.name.clone(),
                    );
                    let target = if alias.as_name.is_some() {
                        module
                    } else {
                        bound.clone()
                    };
                    aliases.insert(bound, target);
                }
            }
            _ => each_block(statement, |block| find_aliases(block, aliases)),
        }
    }
}

fn find_assignments(
    statements: &[Stmt],
    aliases: &BTreeMap<String, String>,
    assigned: &mut BTreeSet<(String, String)>,
) {
    for statement in statements {
        let targets: Vec<&Expr> = match statement {
            Stmt::Assign { targets, .. } => targets.iter().collect(),
            Stmt::AugAssign { target, .. } | Stmt::AnnAssign { target, .. } => vec![target],
            _ => Vec::new(),
        };
        for target in targets {
            if let Expr::Attribute {
                value, attribute, ..
            } = target
                && let Some(module) = module_named(value, aliases)
            {
                assigned.insert((module, attribute.name.clone()));
            }
        }
        each_block(statement, |block| {
            find_assignments(block, aliases, assigned);
        });
    }
}

/// Returns the module a dotted name through an import's name stands for.
fn module_named(expression: &Expr, aliases: &BTreeMap<String, String>) -> Option<String> {
    match expression {
        Expr::Name { id, .. } => aliases.get(id).cloned(),
        Expr::Attribute {
            value, attribute, ..
        } => Some(format!(
            "{}.{}",
            module_named(value, aliases)?,
            attribute.name
        )),
        _ => None,
    }
}

/// Calls `visit` with each block of statements a statement holds.
fn each_block(statement: &Stmt, mut visit: impl FnMut(&[Stmt])) {
    match statement {
        Stmt::FunctionDef { body, .. } | Stmt::ClassDef { body, .. } | Stmt::With { body, .. } => {
            visit(body);
        }
        Stmt::If { body, orelse, .. }
        | Stmt::For { body, orelse, .. }
        | Stmt::While { body, orelse, .. } => {
            visit(body);
            visit(orelse);
        }
        Stmt::Try { .. } => {
            for block in statement.try_blocks() {
                visit(block);
            }
        }
        _ => {}
    }
}
