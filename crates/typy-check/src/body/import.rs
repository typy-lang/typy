//! Resolves `import` and `from ... import` against the modules checked before, and checks
//! the statements that bind their names (SMT-07, MOD-01 to MOD-03, ADR-0015).

use typy_diagnostics::{Diagnostic, DiagnosticCode, RuleId, Span};
use typy_source::syntax::{Alias, Expr, Identifier, Stmt};
use typy_source::{ModuleName, absolute_name};

use super::builtin_module::{BuiltinName, ModuleFunction, is_builtin_module};
use crate::check::{ANNOTATION_MODULES, AnnotationName, annotation_name};

use super::{BodyChecker, Program};
use crate::typed::{
    ClassId, FunctionId, ImportBinding, SymbolId, SymbolKind, TypedExpr, TypedExprKind, TypedStmt,
};
use crate::types::Type;

/// What an import binds a name to.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Imported {
    /// A function of the module, called by the name.
    Function(FunctionId),
    /// A class of the module.
    Class(ClassId),
    /// A module variable of the module, whose value the name takes.
    Variable { module: String, source: SymbolId },
    /// A module object.
    Module(ModuleName),
    /// A name of `typing`, `collections.abc` or `types`, for annotations only (TYP-15).
    Annotation(AnnotationName),
    /// One of those modules, whose names annotations read through it.
    Standard(String),
    /// A function of a built-in module, called by the name (ADR-0018).
    ModuleFunction(super::builtin_module::ModuleFunction),
    /// A value of a built-in module, which the name takes.
    ModuleValue(super::builtin_module::BuiltinValue),
    /// A type alias of another module, which annotations read (SMT-12).
    TypeAlias(std::rc::Rc<crate::check::ExportedAlias>),
}

/// What a name is in a module's tables.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Named {
    Function(FunctionId),
    Class(ClassId),
    Variable(SymbolId),
    /// A type alias, which annotations read, its own module's and those importing it
    /// (SMT-10, SMT-12).
    TypeAlias,
    Nothing,
}

/// An import statement resolved: the modules it initialises, in order, and each name it
/// binds with what the name stands for.
#[derive(Clone, Debug, Default)]
pub(crate) struct Resolved {
    pub(crate) modules: Vec<String>,
    pub(crate) names: Vec<(String, Imported, Span)>,
    /// The lazy imports of other modules whose names this import reads, which run first, as
    /// reading the module's attribute runs them in CPython (MOD-11).
    pub(crate) pending: Vec<FunctionId>,
}

impl Resolved {
    /// Adds a module to initialise, with each package above it first, once.
    fn initialise(&mut self, name: &ModuleName) {
        let mut chain = vec![name.clone()];
        while let Some(parent) = chain.last().and_then(ModuleName::parent) {
            chain.push(parent);
        }
        for module in chain.into_iter().rev() {
            if !self.modules.iter().any(|known| known == module.as_str()) {
                self.modules.push(module.as_str().to_owned());
            }
        }
    }
}

impl Program {
    /// Resolves an import statement of the module being checked, or returns why it cannot
    /// be: a standard library module TyPy does not provide yet, a module checked after this
    /// one, or a name its module does not define.
    pub(crate) fn resolve_import(&self, statement: &Stmt) -> Result<Resolved, Box<Diagnostic>> {
        let mut resolved = Resolved::default();
        match statement {
            Stmt::Import { names, span, .. } => {
                // The parser gives identifiers, so each dotted name is a module name.
                let named = names.iter().filter_map(|alias| {
                    ModuleName::new(&alias.name.name)
                        .ok()
                        .map(|module| (alias, module))
                });
                for (alias, module) in named {
                    if ANNOTATION_MODULES.contains(&module.as_str()) {
                        let (bound, value) = alias.as_name.as_ref().map_or_else(
                            || (module.top().as_str().to_owned(), module.top()),
                            |as_name| (as_name.name.clone(), module.clone()),
                        );
                        resolved.names.push((
                            bound,
                            Imported::Standard(value.as_str().to_owned()),
                            alias.name.span,
                        ));
                        continue;
                    }
                    if let Some(name) = self.builtin_import(alias, &module, *span)? {
                        resolved.names.push(name);
                        continue;
                    }
                    self.check_provided(&module, *span)?;
                    resolved.initialise(&module);
                    // `import a.b` binds `a`; `import a.b as c` binds `c` to `a.b`.
                    let (bound, value) = alias.as_name.as_ref().map_or_else(
                        || {
                            let top = module.top();
                            (top.as_str().to_owned(), top)
                        },
                        |as_name| (as_name.name.clone(), module.clone()),
                    );
                    resolved
                        .names
                        .push((bound, Imported::Module(value), alias.name.span));
                }
            }
            Stmt::ImportFrom {
                module,
                names,
                level,
                span,
                ..
            } => {
                let from = module.as_ref().map(|name| name.name.as_str());
                let base = absolute_name(self.package.as_ref(), *level, from)
                    .map_err(|message| Box::new(imports_nothing(*span, message)))?;
                if ANNOTATION_MODULES.contains(&base.as_str())
                    && !self.loaded.contains(base.as_str())
                {
                    for alias in names {
                        let found = annotation_name(base.as_str(), &alias.name.name)
                            .ok_or_else(|| Box::new(not_given(&base, alias)))?;
                        let bound = alias.as_name.as_ref().unwrap_or(&alias.name);
                        resolved.names.push((
                            bound.name.clone(),
                            Imported::Annotation(found),
                            alias.name.span,
                        ));
                    }
                    return Ok(resolved);
                }
                if is_builtin_module(base.as_str()) && !self.loaded.contains(base.as_str()) {
                    return self.builtin_names(base.as_str(), names, resolved);
                }
                self.check_provided(&base, *span)?;
                resolved.initialise(&base);
                for alias in names {
                    if let Some(reify) = self.lazy_elsewhere(base.as_str(), &alias.name.name) {
                        resolved.pending.push(reify);
                    }
                    let imported = self.imported_name(&base, alias, *span)?;
                    if let Imported::Module(child) = &imported {
                        resolved.initialise(child);
                    }
                    let bound = alias.as_name.as_ref().unwrap_or(&alias.name);
                    resolved
                        .names
                        .push((bound.name.clone(), imported, alias.name.span));
                }
            }
            _ => {}
        }
        Ok(resolved)
    }

    /// Returns why each name a `from` import of `typing`, `collections.abc` or `dataclasses`
    /// binds is refused, or nothing for any other statement.
    pub(crate) fn refused_annotation_names(&self, statement: &Stmt) -> Vec<Diagnostic> {
        let Stmt::ImportFrom {
            module,
            names,
            level,
            ..
        } = statement
        else {
            return Vec::new();
        };
        let from = module.as_ref().map(|name| name.name.as_str());
        let Ok(base) = absolute_name(self.package.as_ref(), *level, from) else {
            return Vec::new();
        };
        if !ANNOTATION_MODULES.contains(&base.as_str()) || self.loaded.contains(base.as_str()) {
            return Vec::new();
        }
        names
            .iter()
            .filter(|alias| annotation_name(base.as_str(), &alias.name.name).is_none())
            .map(|alias| not_given(&base, alias))
            .collect()
    }

    /// Returns the type alias `name` of a module, as an importing module reads it.
    fn alias_value(
        &self,
        module: &str,
        name: &str,
    ) -> Option<std::rc::Rc<crate::check::ExportedAlias>> {
        let aliases = if module == self.module {
            &self.exported_aliases
        } else {
            &self.scopes.get(module)?.alias_values
        };
        aliases.get(name).cloned()
    }

    /// Returns what `name` is in a module checked before, or in the module being checked,
    /// which its own functions may import by name; `None` when the module is not checked
    /// yet. The tables are read in place, so a lookup costs the same in any module.
    pub(crate) fn named(&self, module: &str, name: &str) -> Option<Named> {
        let (functions, classes, symbols, aliases) = if module == self.module {
            (
                &self.function_ids,
                &self.class_ids,
                &self.module_symbols,
                &self.type_aliases,
            )
        } else {
            let scope = self.scopes.get(module)?;
            (
                &scope.function_ids,
                &scope.class_ids,
                &scope.module_symbols,
                &scope.type_aliases,
            )
        };
        Some(if let Some(function) = functions.get(name) {
            Named::Function(*function)
        } else if let Some(class) = classes.get(name) {
            Named::Class(*class)
        } else if let Some(symbol) = symbols.get(name) {
            Named::Variable(*symbol)
        } else if aliases.contains(name) {
            Named::TypeAlias
        } else {
            Named::Nothing
        })
    }

    /// Returns the name `import math` binds to a built-in module, or `None` for any other
    /// module. A built-in module runs no code and has no submodules (ADR-0018).
    fn builtin_import(
        &self,
        alias: &Alias,
        module: &ModuleName,
        span: Span,
    ) -> Result<Option<(String, Imported, Span)>, Box<Diagnostic>> {
        if !is_builtin_module(module.top().as_str()) || self.loaded.contains(module.as_str()) {
            return Ok(None);
        }
        if module.parent().is_some() {
            return Err(Box::new(not_a_package(module, span)));
        }
        let bound = alias.as_name.as_ref().map_or_else(
            || module.as_str().to_owned(),
            |as_name| as_name.name.clone(),
        );
        Ok(Some((
            bound,
            Imported::Module(module.clone()),
            alias.name.span,
        )))
    }

    /// Resolves `from module import names` for a built-in module.
    fn builtin_names(
        &self,
        module: &str,
        names: &[Alias],
        mut resolved: Resolved,
    ) -> Result<Resolved, Box<Diagnostic>> {
        for alias in names {
            let imported = self.builtin_from(module, alias)?;
            let bound = alias.as_name.as_ref().unwrap_or(&alias.name);
            resolved
                .names
                .push((bound.name.clone(), imported, alias.name.span));
        }
        Ok(resolved)
    }

    /// Returns what `from base import name` binds: the module's own name first, as CPython
    /// reads the module's attribute, then a submodule of the program.
    fn imported_name(
        &self,
        base: &ModuleName,
        alias: &Alias,
        span: Span,
    ) -> Result<Imported, Box<Diagnostic>> {
        let name = alias.name.name.as_str();
        let child = base.child(name);
        // A package's own `from . import helper` names a submodule, checked before it.
        if base.as_str() == self.module && self.loaded.contains(child.as_str()) {
            return Ok(Imported::Module(child));
        }
        match self.named(base.as_str(), name) {
            None => return Err(Box::new(checked_later(span, base))),
            Some(Named::Function(function)) => return Ok(Imported::Function(function)),
            Some(Named::Class(class)) => return Ok(Imported::Class(class)),
            Some(Named::Variable(source)) => {
                return Ok(Imported::Variable {
                    module: base.as_str().to_owned(),
                    source,
                });
            }
            Some(Named::TypeAlias) => {
                if let Some(found) = self.alias_value(base.as_str(), name) {
                    return Ok(Imported::TypeAlias(found));
                }
            }
            Some(Named::Nothing) => {}
        }
        if self.loaded.contains(child.as_str()) {
            return Ok(Imported::Module(child));
        }
        if let Some(planned) = self.rebound_elsewhere(base.as_str(), name, alias.name.span) {
            return Err(Box::new(planned));
        }
        Err(Box::new(
            Diagnostic::new(
                DiagnosticCode::error(425),
                alias.name.span,
                format!("module `{base}` has no name `{name}`"),
            )
            .with_rule(RuleId::SMT_07),
        ))
    }

    /// Returns the function of the lazy import by which another module binds `name`, if it
    /// does: reaching the name from outside runs that import first (MOD-11).
    pub(super) fn lazy_elsewhere(&self, module: &str, name: &str) -> Option<FunctionId> {
        if module == self.module {
            return None;
        }
        self.lazy_names
            .get(&(module.to_owned(), name.to_owned()))
            .copied()
    }

    /// Fails for a standard library module, which TyPy provides from a later release or not
    /// at all (LIB-08); a module of the program passes.
    fn check_provided(&self, module: &ModuleName, span: Span) -> Result<(), Box<Diagnostic>> {
        let top = module.as_str().split('.').next().unwrap_or_default();
        // The loader notes each standard library module with the packages above it.
        if !self.standard.contains(top) {
            return Ok(());
        }
        let planned = match top {
            "os" | "time" | "random" => Some(("1.0", RuleId::LIB_07)),
            _ => None,
        };
        Err(Box::new(match planned {
            Some((release, rule)) => Diagnostic::new(
                DiagnosticCode::error(201),
                span,
                format!("the module `{module}` is planned for TyPy {release} ({rule})"),
            )
            .with_rule(rule),
            None => Diagnostic::new(
                DiagnosticCode::error(206),
                span,
                format!("TyPy does not provide the standard library module `{module}`"),
            )
            .with_rule(RuleId::LIB_08)
            .with_help(
                "TyPy provides `math`, `sys`, `dataclasses`, `typing`, `collections.abc` and \
                 `types`",
            ),
        }))
    }
}

/// Returns E0206 for a submodule of a built-in module, which has none.
fn not_a_package(module: &ModuleName, span: Span) -> Diagnostic {
    let top = module.top();
    Diagnostic::new(
        DiagnosticCode::error(206),
        span,
        format!("no module named `{module}`; `{top}` is not a package"),
    )
    .with_rule(RuleId::LIB_08)
}

/// Returns the error for a name of `typing`, `collections.abc`, `types` or `dataclasses` that
/// TyPy does not give: `Any` and `cast` are refused (TYP-10), `NamedTuple` and the abstract
/// collections are planned for a later release, and any other name is not given (LIB-17).
fn not_given(module: &ModuleName, alias: &Alias) -> Diagnostic {
    let name = &alias.name.name;
    if module.as_str() == "types" {
        return Diagnostic::new(
            DiagnosticCode::error(425),
            alias.name.span,
            format!("TyPy's `types` gives `TracebackType` and `ModuleType`, not `{name}`"),
        )
        .with_rule(RuleId::TYP_15);
    }
    // A generic is written with a type parameter list, which needs no `TypeVar` (GEN-01).
    if module.as_str() == "typing" && matches!(name.as_str(), "TypeVar" | "Generic") {
        return Diagnostic::new(
            DiagnosticCode::error(202),
            alias.name.span,
            format!(
                "`typing.{name}` is refused: TyPy writes generic functions and classes with \
                 type parameter lists (GEN-01)"
            ),
        )
        .with_rule(RuleId::GEN_01)
        .with_help("write `def first[T](items: list[T]) -> T:` or `class Box[T]:`");
    }
    // A value of any type, or one forced to a type, would carry its type when the program
    // runs, which TyPy's values do not (TYP-10, TYP-11).
    if module.as_str() == "typing" && matches!(name.as_str(), "Any" | "cast") {
        return Diagnostic::new(
            DiagnosticCode::error(202),
            alias.name.span,
            format!("`typing.{name}` is refused: every value in TyPy has one type (TYP-10)"),
        )
        .with_rule(RuleId::TYP_10)
        .with_help("annotate the value with its type, or a union of its types");
    }
    // The names a later release gives, with the release and the requirement that plan each
    // (ADR-0085, ADR-0086).
    let planned = match name.as_str() {
        "NamedTuple" => Some(("0.10", RuleId::CLS_27)),
        "Sequence" | "MutableSequence" | "Mapping" | "MutableMapping" | "Collection"
        | "Container" | "Set" | "MutableSet" | "Reversible" | "KeysView" | "ValuesView"
        | "ItemsView" => Some(("0.10", RuleId::LIB_20)),
        "AbstractSet" if module.as_str() == "typing" => Some(("0.10", RuleId::LIB_20)),
        "Sized" | "Hashable" | "MappingView" => Some(("0.14", RuleId::LIB_24)),
        _ => None,
    };
    if let Some((release, rule)) = planned {
        return Diagnostic::new(
            DiagnosticCode::error(201),
            alias.name.span,
            format!("`{module}.{name}` is planned for TyPy {release} ({rule})"),
        )
        .with_rule(rule);
    }
    Diagnostic::new(
        DiagnosticCode::error(425),
        alias.name.span,
        format!(
            "TyPy's `{module}` gives {}, not `{name}`",
            listed(given_names(module.as_str()))
        ),
    )
    .with_rule(RuleId::LIB_17)
}

/// Returns the names TyPy gives of `typing`, `collections.abc` or `dataclasses`, each of
/// which `annotation_name` resolves.
fn given_names(module: &str) -> &'static [&'static str] {
    match module {
        "collections.abc" => &["Iterator", "Iterable", "Generator", "Callable"],
        "dataclasses" => &["dataclass", "field"],
        _ => &[
            "Iterator",
            "Iterable",
            "Generator",
            "Callable",
            "Optional",
            "Union",
            "List",
            "Dict",
            "Set",
            "Tuple",
            "Type",
            "Final",
            "ClassVar",
            "Protocol",
        ],
    }
}

/// Returns two or more names written as code and joined as a sentence lists them: `a`, `b`
/// and `c`.
fn listed(names: &[&str]) -> String {
    let quoted: Vec<String> = names.iter().map(|name| format!("`{name}`")).collect();
    let (rest, last) = quoted.split_at(quoted.len() - 1);
    format!("{} and {}", rest.join(", "), last.concat())
}

#[cfg(test)]
mod tests {
    #[test]
    fn each_name_a_refusal_lists_is_one_the_module_gives() {
        for module in ["typing", "collections.abc", "dataclasses"] {
            for name in super::given_names(module) {
                assert!(
                    crate::check::annotation_name(module, name).is_some(),
                    "{module}.{name}"
                );
            }
        }
    }
}

/// Returns the error for a relative import that names no module, with CPython's message.
fn imports_nothing(span: Span, message: &str) -> Diagnostic {
    Diagnostic::new(DiagnosticCode::error(206), span, message).with_rule(RuleId::MOD_02)
}

/// Returns E0426 for an import of a module that is checked after this one inside an
/// `__init__`, which is checked before the rest of its module since it gives the attributes
/// their types; any other function waits for the module (MOD-13).
fn checked_later(span: Span, module: &ModuleName) -> Diagnostic {
    Diagnostic::new(
        DiagnosticCode::error(426),
        span,
        format!(
            "`{module}` imports this module at module level, so importing it inside \
             `__init__` is planned for TyPy 1.0 (MOD-15)"
        ),
    )
    .with_rule(RuleId::MOD_15)
    .with_help("import it at module level in the other direction, or move the shared code to a third module")
}

impl BodyChecker<'_> {
    /// Checks an import statement: each module it initialises, and each name it binds to a
    /// module or to a copy of a module variable's value, typed as an assignment is.
    pub(super) fn import_statement(&mut self, statement: &Stmt, span: Span) -> Option<TypedStmt> {
        let resolved = match self.program.resolve_import(statement) {
            Ok(resolved) => resolved,
            Err(diagnostic) => {
                // Each name `typing` or `collections.abc` does not give is reported, not only
                // the first.
                let refused = self.program.refused_annotation_names(statement);
                if refused.is_empty() {
                    self.diagnostics.push(*diagnostic);
                }
                for each in refused {
                    self.diagnostics.push(each);
                }
                return None;
            }
        };
        let mut bindings = Vec::new();
        let mut lazy_callables = Vec::new();
        let mut numbers = Vec::new();
        for (name, imported, name_span) in resolved.names {
            let ty = match &imported {
                Imported::Function(_) | Imported::Class(_) | Imported::ModuleFunction(_) => {
                    numbers.extend(self.rebound_binding(&name, name_span));
                    lazy_callables.push((name, name_span));
                    continue;
                }
                Imported::ModuleValue(value) => value.expression(name_span).ty,
                Imported::Annotation(_) | Imported::Standard(_) | Imported::TypeAlias(_) => {
                    continue;
                }
                Imported::Variable { source, .. } => {
                    self.program.symbols[source.0 as usize].ty.clone()
                }
                Imported::Module(module) => {
                    if is_builtin_module(module.as_str()) {
                        self.program
                            .builtin_modules
                            .insert(module.as_str().to_owned());
                    }
                    Type::Module(module.as_str().to_owned())
                }
            };
            let target = Expr::Name {
                id: name,
                span: name_span,
            };
            let value = Self::typed(
                TypedExprKind::Constant(crate::typed::TypedConstant::None),
                ty.clone(),
                name_span,
            );
            let binding = move |target| match imported {
                Imported::Variable { module, source } => Some(ImportBinding::Value {
                    target,
                    module,
                    source,
                }),
                Imported::Module(module) => Some(ImportBinding::Module {
                    target,
                    module: module.as_str().to_owned(),
                }),
                Imported::ModuleValue(value) => Some(ImportBinding::Expression {
                    target,
                    value: value.expression(name_span),
                }),
                Imported::Function(_)
                | Imported::Class(_)
                | Imported::Annotation(_)
                | Imported::Standard(_)
                | Imported::TypeAlias(_)
                | Imported::ModuleFunction(_) => None,
            };
            bindings.extend(self.assign(&target, ty, &value).and_then(binding));
        }
        let lazy =
            !self.in_function() && typy_source::is_lazy(statement, &self.program.lazy_modules);
        if !lazy {
            // The import stores its number in each binding variable once its names are bound.
            bindings.append(&mut numbers);
        }
        let import = TypedStmt::Import {
            modules: resolved.modules,
            pending: resolved.pending,
            bindings,
            span,
        };
        if !lazy {
            return Some(import);
        }
        // A lazy import stores its number when its statement runs, as it binds its names then.
        if !numbers.is_empty() {
            self.following.push(TypedStmt::Import {
                modules: Vec::new(),
                pending: Vec::new(),
                bindings: numbers,
                span,
            });
        }
        self.lazy_import(import, lazy_callables, span)
    }

    /// Makes a lazy import of an import statement. Each name it binds gets its own function,
    /// which runs the import for that name alone, so rebinding one name cancels only its own
    /// import, as in CPython. A function or a class several imports bind is called through
    /// its binding variable (ADR-0036).
    fn lazy_import(
        &mut self,
        import: TypedStmt,
        callables: Vec<(String, Span)>,
        span: Span,
    ) -> Option<TypedStmt> {
        let TypedStmt::Import {
            modules,
            pending,
            bindings,
            ..
        } = import
        else {
            return None;
        };
        let mut names: Vec<(String, Option<ImportBinding>, Option<Span>)> = bindings
            .into_iter()
            .map(|binding| {
                let target = binding.target();
                (
                    self.program.symbols[target.0 as usize].name.clone(),
                    Some(binding),
                    None,
                )
            })
            .collect();
        names.extend(
            callables
                .into_iter()
                .map(|(name, name_span)| (name, None, Some(name_span))),
        );
        let mut reifies = Vec::new();
        let mut targets = Vec::new();
        for (name, binding, name_span) in names {
            let target = binding.as_ref().map(ImportBinding::target);
            targets.extend(target);
            let import = TypedStmt::Import {
                modules: modules.clone(),
                pending: pending.clone(),
                bindings: binding.into_iter().collect(),
                span,
            };
            let reify = self
                .program
                .add_lazy_import(name.clone(), import, target, span);
            if let Some(name_span) = name_span {
                self.note_rebound_reify(&name, name_span, reify);
            }
            reifies.push(reify);
        }
        Some(TypedStmt::LazyImport {
            reifies,
            targets,
            span,
        })
    }

    /// Returns the function that runs the lazy import binding `name` in this module, when
    /// it binds it. A variable of a function with the same name hides it.
    pub(super) fn lazy_function(&self, name: &str) -> Option<FunctionId> {
        if let Some(symbol) = self.locals.get(name)
            && self.program.symbols[symbol.0 as usize].kind != SymbolKind::ModuleVariable
        {
            return None;
        }
        let key = (self.program.module.clone(), name.to_owned());
        self.program.lazy_names.get(&key).copied()
    }

    /// Returns the function of the lazy import binding the first name of a module
    /// expression, such as `m` in `m.f()`.
    pub(super) fn lazy_module_function(&self, module: &Expr) -> Option<FunctionId> {
        match module {
            Expr::Name { id, .. } => self.lazy_function(id),
            Expr::Attribute { value, .. } => self.lazy_module_function(value),
            _ => None,
        }
    }

    /// Wraps a call or a read through `name`, so it first runs the lazy import binding the
    /// name while the import waits (MOD-06).
    pub(super) fn through_lazy(&self, name: &str, value: TypedExpr) -> TypedExpr {
        Self::lazy_value(self.lazy_function(name), value)
    }

    /// Wraps a call or a read through a module expression, as `through_lazy` does.
    pub(super) fn through_lazy_module(&self, module: &Expr, value: TypedExpr) -> TypedExpr {
        Self::lazy_value(self.lazy_module_function(module), value)
    }

    fn lazy_value(reify: Option<FunctionId>, value: TypedExpr) -> TypedExpr {
        let Some(reify) = reify else {
            return value;
        };
        let (ty, span) = (value.ty.clone(), value.span);
        Self::typed(
            TypedExprKind::Lazy {
                reify,
                value: Box::new(value),
            },
            ty,
            span,
        )
    }

    /// Binds as locals the names an import binds to values, as `collect_locals` binds an
    /// assignment's targets; a function or a class needs no variable.
    pub(super) fn bind_imported(&mut self, statement: &Stmt) {
        let Ok(resolved) = self.program.resolve_import(statement) else {
            return;
        };
        for (name, imported, span) in resolved.names {
            if matches!(
                imported,
                Imported::Variable { .. } | Imported::Module(_) | Imported::ModuleValue(_)
            ) {
                self.bind_local(&Expr::Name { id: name, span });
            }
        }
    }

    /// Reads `module.name` through a module object: a module variable, read as it is now; a
    /// class, as a type object; or a submodule (MOD-10).
    pub(super) fn module_attribute(&mut self, module: &str, name: &str, span: Span) -> TypedExpr {
        if is_builtin_module(module) {
            let name = Identifier {
                name: name.to_owned(),
                span,
            };
            return self.builtin_attribute(module, &name, span);
        }
        if let Some(reify) = self.program.lazy_elsewhere(module, name) {
            let read = self.module_attribute_now(module, name, span);
            return Self::lazy_value(Some(reify), read);
        }
        self.module_attribute_now(module, name, span)
    }

    /// Reads `module.name` without running a lazy import of it.
    fn module_attribute_now(&mut self, module: &str, name: &str, span: Span) -> TypedExpr {
        let Some(found) = self.program.named(module, name) else {
            let module = ModuleName::new(module).unwrap_or_else(|_| ModuleName::main());
            return self.error(checked_later(span, &module), span);
        };
        if name == "__name__" {
            return Self::typed(
                TypedExprKind::Constant(crate::typed::TypedConstant::Str(module.to_owned())),
                Type::Str,
                span,
            );
        }
        if let Named::Variable(symbol) = found {
            let ty = self.program.symbols[symbol.0 as usize].ty.clone();
            return Self::typed(
                TypedExprKind::ModuleAttribute {
                    module: module.to_owned(),
                    symbol,
                },
                ty,
                span,
            );
        }
        if let Named::Class(class) = found {
            return self.class_object(class, name, span);
        }
        if let Named::TypeAlias = found {
            return self.type_alias_object(module, name, span);
        }
        let child = format!("{module}.{name}");
        if self.program.loaded.contains(&child) {
            return Self::typed(
                TypedExprKind::Submodule(child.clone()),
                Type::Module(child),
                span,
            );
        }
        if let Some(planned) = self.program.rebound_elsewhere(module, name, span) {
            return self.error(planned, span);
        }
        if let Named::Function(function) = found {
            return self.function_value(function, name, span);
        }
        self.error(
            Diagnostic::new(
                DiagnosticCode::error(425),
                span,
                format!("module `{module}` has no name `{name}`"),
            )
            .with_rule(RuleId::SMT_07),
            span,
        )
    }

    /// Reads `module.name` of a built-in module: a value as it is, a function of `math` or
    /// `sys` where a `Callable` is expected (FUN-08), and a `ctypes` function not as a value
    /// yet; a refusal points at the name.
    pub(super) fn builtin_attribute(
        &mut self,
        module: &str,
        attribute: &Identifier,
        span: Span,
    ) -> TypedExpr {
        let name = attribute.name.as_str();
        match self.program.builtin_lookup(module, name, attribute.span) {
            Ok(BuiltinName::Value(value)) => value.expression(span),
            Ok(BuiltinName::Function(super::builtin_module::ModuleFunction::CtypesType(ty))) => {
                Self::cdata_class(crate::ctypes::CData::Scalar(ty), span)
            }
            Ok(BuiltinName::Function(
                super::builtin_module::ModuleFunction::Math(_)
                | super::builtin_module::ModuleFunction::Exit
                | super::builtin_module::ModuleFunction::GetRecursionLimit
                | super::builtin_module::ModuleFunction::SetRecursionLimit,
            )) => self.module_function_without_context(&format!("{module}.{name}"), attribute.span),
            Ok(BuiltinName::Function(_)) => self.error(
                Diagnostic::new(
                    DiagnosticCode::error(201),
                    attribute.span,
                    format!("using `{module}.{name}` as a value is planned for TyPy 0.10 (FUN-08)"),
                )
                .with_rule(RuleId::FUN_08),
                span,
            ),
            Err(diagnostic) => self.error(*diagnostic, span),
        }
    }

    /// Returns the type object of a class as a value, an exception class's for one.
    pub(super) fn class_object(&self, class: ClassId, name: &str, span: Span) -> TypedExpr {
        let of = match self.program.exception(class) {
            Some(exception) => crate::types::TypeObjectOf::Exception(exception.clone()),
            None => crate::types::TypeObjectOf::Instance(class, name.to_owned()),
        };
        let ty = crate::types::TypeObjectOf::type_of(Some(of.clone()));
        Self::typed(TypedExprKind::TypeObject(Box::new(of)), ty, span)
    }

    /// Returns the module a module expression names, when it names one: a variable bound to
    /// a module, or a submodule of one, such as `tools.text`.
    pub(super) fn module_of(&self, expression: &Expr) -> Option<String> {
        match expression {
            Expr::Name { id, .. } => {
                let symbol = self
                    .locals
                    .get(id)
                    .or_else(|| self.program.module_symbols.get(id))?;
                let ty = self
                    .types
                    .get(symbol)
                    .unwrap_or(&self.program.symbols[symbol.0 as usize].ty);
                match ty {
                    Type::Module(module) => Some(module.clone()),
                    _ => None,
                }
            }
            Expr::Attribute {
                value, attribute, ..
            } => {
                let child = format!("{}.{}", self.module_of(value)?, attribute.name);
                self.program.loaded.contains(&child).then_some(child)
            }
            _ => None,
        }
    }

    /// Checks `module.name(arguments)`: a call of the module's function, or of its class,
    /// which makes an instance or an exception.
    pub(super) fn module_call(
        &mut self,
        module: &str,
        name: &typy_source::syntax::Identifier,
        arguments: &[Expr],
        keywords: &[typy_source::syntax::Keyword],
        span: Span,
    ) -> TypedExpr {
        if is_builtin_module(module) {
            return match self.program.builtin_lookup(module, &name.name, name.span) {
                Ok(BuiltinName::Function(function)) => {
                    self.module_function_call(function, arguments, keywords, span)
                }
                Ok(BuiltinName::Value(value)) => {
                    let ty = value.expression(name.span).ty;
                    self.error(
                        Diagnostic::new(
                            DiagnosticCode::error(401),
                            name.span,
                            format!(
                                "`{module}.{}` is {} and cannot be called",
                                name.name,
                                ty.with_article()
                            ),
                        )
                        .with_rule(RuleId::EXP_02),
                        span,
                    )
                }
                Err(diagnostic) => self.error(*diagnostic, span),
            };
        }
        if let Some(reify) = self.program.lazy_elsewhere(module, &name.name) {
            let call = self.module_call_now(module, name, arguments, keywords, span);
            return Self::lazy_value(Some(reify), call);
        }
        self.module_call_now(module, name, arguments, keywords, span)
    }

    /// Checks `module.name(arguments)` without running a lazy import of `name`.
    fn module_call_now(
        &mut self,
        module: &str,
        name: &typy_source::syntax::Identifier,
        arguments: &[Expr],
        keywords: &[typy_source::syntax::Keyword],
        span: Span,
    ) -> TypedExpr {
        match self.program.named(module, &name.name) {
            None => {
                let module = ModuleName::new(module).unwrap_or_else(|_| ModuleName::main());
                return self.error(checked_later(span, &module), span);
            }
            Some(Named::Function(function)) => {
                return self.call_function(function, &name.name, arguments, keywords, span);
            }
            Some(Named::Class(class)) => {
                return self.new_instance(class, arguments, keywords, span);
            }
            Some(Named::Variable(_) | Named::TypeAlias | Named::Nothing) => {}
        }
        let read = self.module_attribute(module, &name.name, name.span);
        // A module variable bound to a `ctypes` class makes an object of it (FFI-03).
        if let Type::CDataClass(class) = &read.ty {
            return self.cdata_new(class, arguments, keywords, span);
        }
        for argument in arguments {
            self.expression(argument);
        }
        if read.ty == Type::Error {
            return TypedExpr { span, ..read };
        }
        self.error(
            Diagnostic::new(
                DiagnosticCode::error(401),
                name.span,
                format!(
                    "`{module}.{}` is {} and cannot be called",
                    name.name,
                    read.ty.with_article()
                ),
            )
            .with_rule(RuleId::EXP_02),
            span,
        )
    }

    /// Checks `module.name = value`: the module variable takes the value, of its type.
    pub(super) fn module_attribute_target(
        &mut self,
        module: &str,
        attribute: &typy_source::syntax::Identifier,
        ty: &Type,
        value: &TypedExpr,
        span: Span,
    ) -> Option<crate::typed::AssignTarget> {
        let read = self.module_attribute(module, &attribute.name, span);
        let TypedExprKind::ModuleAttribute { symbol, .. } = read.kind else {
            if read.ty != Type::Error {
                self.diagnostics.push(
                    Diagnostic::new(
                        DiagnosticCode::error(402),
                        span,
                        format!("`{module}.{}` is not a module variable", attribute.name),
                    )
                    .with_rule(RuleId::SMT_07),
                );
            }
            return None;
        };
        if self.final_reassigned(symbol, &attribute.name, span) {
            return None;
        }
        if !ty.is_assignable_to(&read.ty) {
            self.diagnostics.push(
                Diagnostic::new(
                    DiagnosticCode::error(402),
                    value.span,
                    format!(
                        "`{module}.{}` was declared as {} and is assigned {}",
                        attribute.name,
                        read.ty,
                        ty.with_article()
                    ),
                )
                .with_rule(RuleId::TYP_03),
            );
            return None;
        }
        Some(crate::typed::AssignTarget::Name(symbol))
    }

    /// Adds the functions and classes a function's own imports bind by name to the tables
    /// its body reads, and returns what they replaced, which `restore_names` puts back.
    pub(super) fn import_names_of(&mut self, body: &[Stmt]) -> SavedNames {
        let mut saved = SavedNames::default();
        let mut statements = Vec::new();
        imports_in(body, &mut statements);
        for statement in statements {
            let Ok(resolved) = self.program.resolve_import(statement) else {
                continue;
            };
            for (name, imported, _) in resolved.names {
                // A function's own import of a name hides the module's lazy import of it, and
                // the module's binding variable for it.
                let key = (self.program.module.clone(), name.clone());
                if let Some(lazy) = self.program.lazy_names.remove(&key) {
                    saved.lazy.push((name.clone(), lazy));
                }
                if let Some(rebound) = self.program.rebound.remove(&name) {
                    saved.rebound.push((name.clone(), rebound));
                }
                match imported {
                    Imported::Function(function) => saved.functions.push((
                        name.clone(),
                        self.program.function_ids.insert(name, function),
                    )),
                    Imported::Class(class) => saved
                        .classes
                        .push((name.clone(), self.program.class_ids.insert(name, class))),
                    Imported::Annotation(found) => saved.annotations.push((
                        name.clone(),
                        self.program.annotation_names.insert(name, found),
                    )),
                    Imported::Standard(module) => saved.standard.push((
                        name.clone(),
                        self.program.annotation_modules.insert(name, module),
                    )),
                    Imported::ModuleFunction(function) => saved.math.push((
                        name.clone(),
                        self.program.module_functions.insert(name, function),
                    )),
                    Imported::TypeAlias(alias) => saved.aliases.push((
                        name.clone(),
                        self.program.imported_aliases.insert(name, alias),
                    )),
                    Imported::Variable { .. } | Imported::Module(_) | Imported::ModuleValue(_) => {}
                }
            }
        }
        saved
    }

    /// Puts back the names `import_names_of` replaced, newest first.
    pub(super) fn restore_names(&mut self, saved: SavedNames) {
        for (name, previous) in saved.aliases.into_iter().rev() {
            match previous {
                Some(alias) => self.program.imported_aliases.insert(name, alias),
                None => self.program.imported_aliases.remove(&name),
            };
        }
        for (name, previous) in saved.math.into_iter().rev() {
            match previous {
                Some(function) => self.program.module_functions.insert(name, function),
                None => self.program.module_functions.remove(&name),
            };
        }
        for (name, lazy) in saved.lazy {
            let key = (self.program.module.clone(), name);
            self.program.lazy_names.insert(key, lazy);
        }
        for (name, rebound) in saved.rebound {
            self.program.rebound.insert(name, rebound);
        }
        for (name, previous) in saved.functions.into_iter().rev() {
            match previous {
                Some(function) => self.program.function_ids.insert(name, function),
                None => self.program.function_ids.remove(&name),
            };
        }
        for (name, previous) in saved.classes.into_iter().rev() {
            match previous {
                Some(class) => self.program.class_ids.insert(name, class),
                None => self.program.class_ids.remove(&name),
            };
        }
        for (name, previous) in saved.annotations.into_iter().rev() {
            match previous {
                Some(found) => self.program.annotation_names.insert(name, found),
                None => self.program.annotation_names.remove(&name),
            };
        }
        for (name, previous) in saved.standard.into_iter().rev() {
            match previous {
                Some(module) => self.program.annotation_modules.insert(name, module),
                None => self.program.annotation_modules.remove(&name),
            };
        }
    }
}

/// The names a function's imports replaced in the module's tables.
#[derive(Default)]
pub(super) struct SavedNames {
    functions: Vec<(String, Option<FunctionId>)>,
    classes: Vec<(String, Option<ClassId>)>,
    annotations: Vec<(String, Option<AnnotationName>)>,
    standard: Vec<(String, Option<String>)>,
    lazy: Vec<(String, FunctionId)>,
    math: Vec<(String, Option<ModuleFunction>)>,
    aliases: Vec<(String, Option<std::rc::Rc<crate::check::ExportedAlias>>)>,
    rebound: Vec<(String, super::rebound::Rebound)>,
}

impl Program {
    /// Returns true when a function body imports a module of the program that is not
    /// checked yet, so the body waits until it is (MOD-13).
    pub(crate) fn imports_later_modules(&self, body: &[Stmt]) -> bool {
        let mut statements = Vec::new();
        imports_in(body, &mut statements);
        statements
            .iter()
            .any(|statement| self.imports_later_module(statement))
    }
}

/// Collects the import statements of a block, at any depth outside nested definitions.
fn imports_in<'tree>(statements: &'tree [Stmt], found: &mut Vec<&'tree Stmt>) {
    for statement in statements {
        match statement {
            Stmt::Import { .. } | Stmt::ImportFrom { .. } => found.push(statement),
            Stmt::If { body, orelse, .. }
            | Stmt::For { body, orelse, .. }
            | Stmt::While { body, orelse, .. } => {
                imports_in(body, found);
                imports_in(orelse, found);
            }
            Stmt::With { body, .. } => imports_in(body, found),
            Stmt::Match { cases, .. } => {
                for case in cases {
                    imports_in(&case.body, found);
                }
            }
            Stmt::Try { .. } => {
                for block in statement.try_blocks() {
                    imports_in(block, found);
                }
            }
            _ => {}
        }
    }
}
