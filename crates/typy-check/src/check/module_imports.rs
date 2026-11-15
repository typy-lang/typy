//! The names a module's module-level imports bind, which its tables and annotations need
//! before its functions and classes are collected, and the module variables assigned through
//! module objects anywhere in the program (SMT-07, ADR-0015).

use std::collections::{BTreeMap, BTreeSet};

use typy_diagnostics::{Diagnostic, DiagnosticCode, Diagnostics, RuleId, Span};
use typy_source::syntax::{Expr, Stmt};

use crate::body::{Binding, Bound, Imported, Program};
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
    /// Each import that binds a name to a function or a class, in source order.
    pub(super) bound: BTreeMap<String, Vec<Binding>>,
    /// The names an import inside a block, such as an `if` or a `try`, binds to a function
    /// or a class.
    pub(super) in_blocks: BTreeSet<String>,
}

/// Returns what the imports at module level bind, in any block outside a `def` or a `class`.
/// An import that cannot be resolved binds nothing here; the module body reports it.
pub(super) fn module_imports(program: &Program, statements: &[Stmt]) -> ModuleImports {
    let mut found = ModuleImports::default();
    collect(program, statements, &mut found, false);
    found
}

fn collect(program: &Program, statements: &[Stmt], found: &mut ModuleImports, in_block: bool) {
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
                            found.bind(&name, Bound::Function(function), lazy, span, in_block);
                            found.functions.insert(name, function);
                        }
                        Imported::Class(class) => {
                            found.bind(&name, Bound::Class(class), lazy, span, in_block);
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
                            found.bind(
                                &name,
                                Bound::ModuleFunction(function),
                                lazy,
                                span,
                                in_block,
                            );
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
                collect(program, body, found, true);
                collect(program, orelse, found, true);
            }
            Stmt::With { body, .. } => collect(program, body, found, true),
            Stmt::Match { cases, .. } => {
                for case in cases {
                    collect(program, &case.body, found, true);
                }
            }
            Stmt::Try { .. } => {
                for block in statement.try_blocks() {
                    collect(program, block, found, true);
                }
            }
            _ => {}
        }
    }
}

impl ModuleImports {
    /// Notes that an import binds `name` to `target`.
    fn bind(&mut self, name: &str, target: Bound, lazy: bool, span: Span, in_block: bool) {
        if in_block {
            self.in_blocks.insert(name.to_owned());
        }
        self.bound
            .entry(name.to_owned())
            .or_default()
            .push(Binding { target, span, lazy });
    }

    /// Returns the names whose calls test a binding variable: those imports bind to more
    /// than one function or class, and those an import inside a block binds (ADR-0036).
    pub(super) fn rebound(&self) -> Vec<(String, Vec<Binding>)> {
        self.bound
            .iter()
            .filter(|(name, bindings)| {
                self.in_blocks.contains(*name)
                    || bindings
                        .iter()
                        .any(|binding| binding.target != bindings[0].target)
            })
            .map(|(name, bindings)| (name.clone(), bindings.clone()))
            .collect()
    }

    /// Takes the names imports bind to more than one function or class out of the tables of
    /// functions and classes, so only a call through the binding variable reads them, and
    /// adds each binding variable to the module's variables.
    pub(super) fn add_binding_variables(&mut self) {
        for (name, bindings) in self.rebound() {
            if bindings
                .iter()
                .any(|binding| binding.target != bindings[0].target)
            {
                self.functions.remove(&name);
                self.classes.remove(&name);
                self.module_functions.remove(&name);
                self.annotations
                    .insert(name.clone(), crate::check::AnnotationName::Rebound);
            }
            self.variables
                .insert(crate::body::binding_variable(&name), bindings[0].span);
        }
    }

    /// Reports each name an import binds to a function or a class that the module also
    /// defines with `def` or `class`, and each one an import binds to a function that the
    /// module also assigns as a variable.
    pub(super) fn report_clashes(
        &self,
        defined: &[String],
        variables: &BTreeMap<String, Span>,
        diagnostics: &mut Diagnostics,
    ) {
        for name in defined {
            if let Some(bindings) = self.bound.get(name) {
                diagnostics.push(bound_twice(name, bindings[0].span));
            }
        }
        self.report_assigned(variables, diagnostics);
    }
}

impl ModuleImports {
    /// Returns the names only imports outside any block bind to a function or a class, which
    /// a statement may reach only after such an import runs.
    pub(super) fn bound_at_top(&self) -> BTreeSet<String> {
        let rebound: BTreeSet<String> = self.rebound().into_iter().map(|(name, _)| name).collect();
        self.bound
            .keys()
            .filter(|name| !rebound.contains(*name))
            .cloned()
            .collect()
    }

    /// Reports each name an import binds to a function that the module also assigns as a
    /// variable, as a `def` of the name is; a class is reported with the module's own.
    fn report_assigned(&self, variables: &BTreeMap<String, Span>, diagnostics: &mut Diagnostics) {
        for (name, bindings) in &self.bound {
            if let (Bound::Function(_) | Bound::ModuleFunction(_), Some(span)) =
                (bindings[0].target, variables.get(name))
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

/// Returns the error for a name an import binds that the module also defines.
fn bound_twice(name: &str, span: Span) -> Diagnostic {
    Diagnostic::new(
        DiagnosticCode::error(202),
        span,
        format!(
            "`{name}` is bound by an import and defined by the module; TyPy keeps one \
             definition per name"
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
