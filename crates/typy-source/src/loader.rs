//! Finds the modules a program imports, the files they are read from, and the order in which
//! the module-level imports initialise them (MOD-01, MOD-02, MOD-05, ADR-0015).

use std::collections::{BTreeMap, BTreeSet};

use camino::{Utf8Path, Utf8PathBuf};
use typy_diagnostics::{Diagnostic, DiagnosticCode, RuleId, Span};

use crate::module::{Module, ModuleName};
use crate::settings::ProjectSettings;
use crate::syntax::{Alias, Stmt};

/// The standard library modules TyPy 0.3 gives without a file, whose names serve annotations
/// only (TYP-15, ADR-0015).
pub const ANNOTATION_MODULES: [&str; 4] = ["collections", "collections.abc", "types", "typing"];

/// Returns whether a module belongs to CPython's standard library, by its top-level name.
pub(crate) fn is_standard(name: &ModuleName) -> bool {
    let top = name.as_str().split('.').next().unwrap_or_default();
    crate::stdlib_names::STDLIB_NAMES
        .binary_search(&top)
        .is_ok()
}

/// One module an import statement needs: the module it names or a package above it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Import {
    /// The module, by its absolute name.
    pub module: ModuleName,
    /// The import statement.
    pub span: Span,
    /// Whether the import runs while its module initialises: at module level or in a class
    /// body, not inside a function.
    pub at_module_level: bool,
    /// Whether the import needs the module only if it exists: `from a import b` imports the
    /// submodule `a.b` only when `a` has no `b` of its own.
    pub optional: bool,
}

/// Returns the absolute name of the module a relative import names: `level` dots counted
/// from `package`, the importing module's package, then `name`. A module outside a package,
/// such as `__main__`, has no package to start from, and too many dots leave the top
/// package; both give CPython's message.
pub fn absolute_name(
    package: Option<&ModuleName>,
    level: u32,
    name: Option<&str>,
) -> Result<ModuleName, &'static str> {
    if level == 0 {
        return name
            .and_then(|name| ModuleName::new(name).ok())
            .ok_or("an import names a module");
    }
    let mut base = package
        .cloned()
        .ok_or("attempted relative import with no known parent package")?;
    for _ in 1..level {
        base = base
            .parent()
            .ok_or("attempted relative import beyond top-level package")?;
    }
    match name {
        Some(name) => {
            ModuleName::new(&format!("{base}.{name}")).map_err(|_| "an import names a module")
        }
        None => Ok(base),
    }
}

/// Returns every module the imports of `module` need, in source order, with the packages
/// above each; an import that names no module it can find gives a diagnostic.
pub(crate) fn imports(module: &Module) -> (Vec<Import>, Vec<Diagnostic>) {
    let mut found = Collector {
        lazy: crate::lazy::lazy_modules(&module.syntax.body),
        ..Collector::default()
    };
    found.statements(module, &module.syntax.body, true);
    (found.imports, found.diagnostics)
}

#[derive(Default)]
struct Collector {
    imports: Vec<Import>,
    diagnostics: Vec<Diagnostic>,
    /// The modules `__lazy_modules__` lists.
    lazy: std::collections::BTreeSet<String>,
}

impl Collector {
    fn statements(&mut self, module: &Module, statements: &[Stmt], at_module_level: bool) {
        for statement in statements {
            self.statement(module, statement, at_module_level);
        }
    }

    fn statement(&mut self, module: &Module, statement: &Stmt, at_module_level: bool) {
        // A lazy import runs when its name is first used, not while its module initialises.
        let at_module_level = at_module_level && !crate::lazy::is_lazy(statement, &self.lazy);
        match statement {
            Stmt::Import { names, span, .. } => {
                // The parser gives identifiers, so each dotted name is a module name.
                for name in names
                    .iter()
                    .filter_map(|alias| ModuleName::new(&alias.name.name).ok())
                {
                    self.with_parents(&name, *span, at_module_level, false);
                }
            }
            Stmt::ImportFrom {
                module: from,
                names,
                level,
                span,
                ..
            } => {
                let from_name = from.as_ref().map(|name| name.name.as_str());
                match absolute_name(module.package().as_ref(), *level, from_name) {
                    Ok(base) => self.imported_names(&base, names, *span, at_module_level),
                    Err(message) => self.diagnostics.push(not_found(*span, message.to_owned())),
                }
            }
            Stmt::FunctionDef { body, .. } => self.statements(module, body, false),
            Stmt::If { body, orelse, .. }
            | Stmt::For { body, orelse, .. }
            | Stmt::While { body, orelse, .. } => {
                self.statements(module, body, at_module_level);
                self.statements(module, orelse, at_module_level);
            }
            // A class body runs while its module initialises, as a `with` body does.
            Stmt::ClassDef { body, .. } | Stmt::With { body, .. } => {
                self.statements(module, body, at_module_level);
            }
            Stmt::Try { .. } => {
                for block in statement.try_blocks() {
                    self.statements(module, block, at_module_level);
                }
            }
            _ => {}
        }
    }

    /// Notes `from base import names`: the module `base` with its packages, and each name as
    /// a submodule that is imported when it exists.
    fn imported_names(
        &mut self,
        base: &ModuleName,
        names: &[Alias],
        span: Span,
        at_module_level: bool,
    ) {
        self.with_parents(base, span, at_module_level, false);
        let children = names
            .iter()
            .filter(|alias| alias.name.name != "*")
            .filter_map(|alias| ModuleName::new(&format!("{base}.{}", alias.name.name)).ok());
        for child in children {
            self.add(child, span, at_module_level, true);
        }
    }

    /// Notes a module and each package above it, from the top, as CPython initialises them.
    fn with_parents(
        &mut self,
        name: &ModuleName,
        span: Span,
        at_module_level: bool,
        optional: bool,
    ) {
        let mut chain = vec![name.clone()];
        while let Some(parent) = chain.last().and_then(ModuleName::parent) {
            chain.push(parent);
        }
        for module in chain.into_iter().rev() {
            self.add(module, span, at_module_level, optional);
        }
    }

    fn add(&mut self, module: ModuleName, span: Span, at_module_level: bool, optional: bool) {
        self.imports.push(Import {
            module,
            span,
            at_module_level,
            optional,
        });
    }
}

/// Where a module's source is: the file under `root`, and whether it is a package.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Location {
    pub(crate) path: Utf8PathBuf,
    pub(crate) is_package: bool,
}

/// Finds a module in the root, then in each module path in order, as CPython searches the
/// script's directory, then `PYTHONPATH` (MOD-01). In each directory the package
/// `a/b/__init__.py` comes before the file `a/b.py`, as CPython's file finder prefers it.
pub(crate) fn locate(settings: &ProjectSettings, name: &ModuleName) -> Option<Location> {
    let relative: Utf8PathBuf = name.as_str().split('.').collect();
    std::iter::once(Utf8PathBuf::new())
        .chain(settings.module_paths.iter().cloned())
        .find_map(|directory| locate_in(&settings.root, &directory.join(&relative)))
}

/// Finds the package or the module at `relative` under `root`.
fn locate_in(root: &Utf8Path, relative: &Utf8Path) -> Option<Location> {
    let package = relative.join("__init__.py");
    if root.join(&package).is_file() {
        return Some(Location {
            path: package,
            is_package: true,
        });
    }
    let file = relative.with_extension("py");
    root.join(&file).is_file().then_some(Location {
        path: file,
        is_package: false,
    })
}

/// Returns the error for an import that names no module the program or TyPy has, with the
/// standard library modules TyPy provides (LIB-08).
pub(crate) fn not_found(span: Span, message: String) -> Diagnostic {
    Diagnostic::new(DiagnosticCode::error(206), span, message)
        .with_rule(RuleId::LIB_08)
        .with_help(format!(
            "a module is a `.py` file or a package under the main file's directory; TyPy's standard library gives {}",
            ANNOTATION_MODULES.join(", ")
        ))
}

/// Returns the modules in the order a module's dependencies come before it, and a diagnostic
/// for each cycle of the imports that run while modules initialise (MOD-05). An import inside
/// a function orders its module first too, unless that would close a cycle.
pub(crate) fn initialisation_order(
    modules: &BTreeMap<ModuleName, Vec<Import>>,
) -> (Vec<ModuleName>, Vec<Diagnostic>) {
    let mut order = Vec::new();
    let mut diagnostics = Vec::new();
    let mut done = BTreeSet::new();
    let mut visiting = BTreeSet::new();
    for name in modules.keys() {
        let mut path = Vec::new();
        let mut walk = Walk {
            modules,
            done: &mut done,
            visiting: &mut visiting,
            order: &mut order,
            diagnostics: &mut diagnostics,
        };
        walk.visit(name, &mut path);
    }
    (order, diagnostics)
}

/// What a depth-first walk over the imports keeps: the modules done and the ones being
/// visited, the order modules initialise in, and the cycles found.
struct Walk<'walk> {
    modules: &'walk BTreeMap<ModuleName, Vec<Import>>,
    done: &'walk mut BTreeSet<ModuleName>,
    visiting: &'walk mut BTreeSet<ModuleName>,
    order: &'walk mut Vec<ModuleName>,
    diagnostics: &'walk mut Vec<Diagnostic>,
}

impl Walk<'_> {
    /// Visits a module's imports depth first; `path` holds the module-level imports being
    /// followed, so a module-level import of a module already on it closes a cycle. An import
    /// inside a function, or a lazy one, runs once its module has initialised, so it adds no
    /// edge to a cycle (OI-21): the module it imports starts a path of its own.
    fn visit(&mut self, name: &ModuleName, path: &mut Vec<(ModuleName, Span)>) {
        if self.done.contains(name) {
            return;
        }
        if let Some(start) = path.iter().position(|(on_path, _)| on_path == name) {
            self.diagnostics.push(cycle(&path[start..]));
            return;
        }
        // Reached again through an import that adds no edge, while it is still visited.
        if !self.visiting.insert(name.clone()) {
            return;
        }
        let imports = self
            .modules
            .get(name)
            .map(Vec::as_slice)
            .unwrap_or_default();
        // Module-level imports come first, so they decide the order modules initialise in.
        let (module_level, later): (Vec<&Import>, Vec<&Import>) =
            imports.iter().partition(|import| import.at_module_level);
        for import in module_level.into_iter().chain(later) {
            if import.module == *name || !self.modules.contains_key(&import.module) {
                continue;
            }
            if import.at_module_level {
                path.push((name.clone(), import.span));
                self.visit(&import.module, path);
                path.pop();
            } else {
                self.visit(&import.module, &mut Vec::new());
            }
        }
        self.visiting.remove(name);
        self.done.insert(name.clone());
        self.order.push(name.clone());
    }
}

/// Returns E0422 for a cycle of module-level imports, each import shown (MOD-05).
fn cycle(path: &[(ModuleName, Span)]) -> Diagnostic {
    let (first, span) = &path[0];
    let mut names: Vec<String> = path.iter().map(|(name, _)| format!("`{name}`")).collect();
    names.push(format!("`{first}`"));
    let mut diagnostic = Diagnostic::new(
        DiagnosticCode::error(422),
        *span,
        format!(
            "circular import: {} imports {}",
            names[0],
            names[1..].join(", which imports ")
        ),
    )
    .with_rule(RuleId::MOD_05)
    .with_help("move one of these imports into the function that uses it");
    for (name, span) in &path[1..] {
        diagnostic = diagnostic.with_label(*span, format!("`{name}` imports here"));
    }
    diagnostic
}
