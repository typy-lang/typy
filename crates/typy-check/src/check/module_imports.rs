//! The names a module's module-level imports bind, which its tables and annotations need
//! before its functions and classes are collected, and the module variables assigned through
//! module objects anywhere in the program (SMT-07, ADR-0015).

use std::collections::{BTreeMap, BTreeSet};

use typy_diagnostics::{Diagnostic, DiagnosticCode, Diagnostics, RuleId, Span};
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
    /// The functions of built-in modules imported by name.
    pub(super) module_functions: BTreeMap<String, crate::body::ModuleFunction>,
    /// The type aliases of other modules imported by name (SMT-12).
    pub(super) aliases: crate::check::AliasTable,
    /// Each name an import binds to a function or a class, with what it binds, whether the
    /// import is lazy, and where.
    pub(super) bound: BTreeMap<String, (Bound, bool, Span)>,
    /// A name bound to two functions or classes, which TyPy refuses since one name keeps one
    /// definition.
    pub(super) clashes: Vec<Diagnostic>,
}

/// What an import binds a name to that a call or an annotation reads when compiling.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Bound {
    Function(FunctionId),
    Class(ClassId),
    ModuleFunction(crate::body::ModuleFunction),
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
                let lazy = typy_source::is_lazy(statement, program.lazy_modules());
                for (name, imported, span) in resolved.names {
                    match imported {
                        Imported::Function(function) => {
                            found.bind(&name, Bound::Function(function), lazy, span);
                            found.functions.insert(name, function);
                        }
                        Imported::Class(class) => {
                            found.bind(&name, Bound::Class(class), lazy, span);
                            found.classes.insert(name, class);
                        }
                        Imported::Module(module) => {
                            found.variables.entry(name.clone()).or_insert(span);
                            found.modules.insert(name, module.as_str().to_owned());
                        }
                        Imported::Variable { .. } | Imported::ModuleValue(_) => {
                            found.variables.entry(name).or_insert(span);
                        }
                        Imported::Annotation(annotation) => {
                            found.annotations.insert(name, annotation);
                        }
                        Imported::Standard(module) => {
                            found.standard.insert(name, module);
                        }
                        Imported::ModuleFunction(function) => {
                            found.bind(&name, Bound::ModuleFunction(function), lazy, span);
                            found.module_functions.insert(name, function);
                        }
                        Imported::TypeAlias(alias) => {
                            found.aliases.insert(name, alias);
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
            Stmt::Match { cases, .. } => {
                for case in cases {
                    collect(program, &case.body, found);
                }
            }
            Stmt::Try { .. } => {
                for block in statement.try_blocks() {
                    collect(program, block, found);
                }
            }
            _ => {}
        }
    }
}

impl ModuleImports {
    /// Notes that an import binds `name` to `target`, and reports a name an earlier import
    /// bound to another function or class: a call compiled for one would run the other.
    /// Two lazy imports of one name are left to the lazy import's own check (MOD-12).
    fn bind(&mut self, name: &str, target: Bound, lazy: bool, span: Span) {
        if let Some((earlier, earlier_lazy, _)) = self.bound.get(name)
            && *earlier != target
            && !(lazy && *earlier_lazy)
        {
            self.clashes.push(bound_twice(name, span));
        }
        self.bound.insert(name.to_owned(), (target, lazy, span));
    }

    /// Reports each name imports bind to two functions or classes, each one an import binds
    /// to a function or a class that the module also defines with `def` or `class`, and each
    /// one an import binds to a function that the module also assigns as a variable.
    pub(super) fn report_clashes(
        &self,
        defined: &[String],
        variables: &BTreeMap<String, Span>,
        diagnostics: &mut Diagnostics,
    ) {
        for clash in &self.clashes {
            diagnostics.push(clash.clone());
        }
        for name in defined {
            if let Some((_, _, span)) = self.bound.get(name) {
                diagnostics.push(bound_twice(name, *span));
            }
        }
        self.report_assigned(variables, diagnostics);
    }
}

impl ModuleImports {
    /// Reports each name an import binds to a function that the module also assigns as a
    /// variable, as a `def` of the name is; a class is reported with the module's own.
    fn report_assigned(&self, variables: &BTreeMap<String, Span>, diagnostics: &mut Diagnostics) {
        for (name, (bound, _, _)) in &self.bound {
            if let (Bound::Function(_) | Bound::ModuleFunction(_), Some(span)) =
                (bound, variables.get(name))
            {
                diagnostics.push(
                    Diagnostic::new(
                        DiagnosticCode::error(202),
                        *span,
                        format!("`{name}` is a function and is also assigned as a variable"),
                    )
                    .with_rule(RuleId::STD_04),
                );
            }
        }
    }
}

/// Returns the error for a name bound to two functions or classes.
fn bound_twice(name: &str, span: Span) -> Diagnostic {
    Diagnostic::new(
        DiagnosticCode::error(202),
        span,
        format!(
            "`{name}` is bound to more than one function or class; TyPy keeps one definition \
             per name"
        ),
    )
    .with_rule(RuleId::STD_04)
    .with_help("import it under another name with `as`")
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
        Stmt::Match { cases, .. } => {
            for case in cases {
                visit(&case.body);
            }
        }
        Stmt::Try { .. } => {
            for block in statement.try_blocks() {
                visit(block);
            }
        }
        _ => {}
    }
}
