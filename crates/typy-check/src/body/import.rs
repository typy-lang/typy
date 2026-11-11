//! Resolves `import` and `from ... import` against the modules checked before, and checks
//! the statements that bind their names (SMT-07, MOD-01 to MOD-03, ADR-0015).

use typy_diagnostics::{Diagnostic, DiagnosticCode, RuleId, Span};
use typy_source::syntax::{Alias, Expr, Stmt};
use typy_source::{ModuleName, absolute_name};

use crate::check::{ANNOTATION_MODULES, AnnotationName, annotation_name};

use super::{BodyChecker, Program};
use crate::typed::{
    ClassId, FunctionId, ImportBinding, SymbolId, SymbolKind, TypedExpr, TypedExprKind, TypedStmt,
};
use crate::types::Type;

/// What an import binds a name to.
#[derive(Clone, Debug, PartialEq, Eq)]
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
}

/// What a name is in a module's tables.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Named {
    Function(FunctionId),
    Class(ClassId),
    Variable(SymbolId),
    Nothing,
}

/// An import statement resolved: the modules it initialises, in order, and each name it
/// binds with what the name stands for.
#[derive(Clone, Debug, Default)]
pub(crate) struct Resolved {
    pub(crate) modules: Vec<String>,
    pub(crate) names: Vec<(String, Imported, Span)>,
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
                self.check_provided(&base, *span)?;
                resolved.initialise(&base);
                for alias in names {
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

    /// Returns what `name` is in a module checked before, or in the module being checked,
    /// which its own functions may import by name; `None` when the module is not checked
    /// yet. The tables are read in place, so a lookup costs the same in any module.
    pub(crate) fn named(&self, module: &str, name: &str) -> Option<Named> {
        let (functions, classes, symbols) = if module == self.module {
            (&self.function_ids, &self.class_ids, &self.module_symbols)
        } else {
            let scope = self.scopes.get(module)?;
            (&scope.function_ids, &scope.class_ids, &scope.module_symbols)
        };
        Some(if let Some(function) = functions.get(name) {
            Named::Function(*function)
        } else if let Some(class) = classes.get(name) {
            Named::Class(*class)
        } else if let Some(symbol) = symbols.get(name) {
            Named::Variable(*symbol)
        } else {
            Named::Nothing
        })
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
        self.check_not_lazy(base.as_str(), name, alias.name.span)?;
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
            Some(Named::Nothing) => {}
        }
        if self.loaded.contains(child.as_str()) {
            return Ok(Imported::Module(child));
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

    /// Fails for a name another module binds by a lazy import: reaching it from outside
    /// would run that import first, which is planned for 0.4 (MOD-11).
    pub(super) fn check_not_lazy(
        &self,
        module: &str,
        name: &str,
        span: Span,
    ) -> Result<(), Box<Diagnostic>> {
        if module == self.module
            || !self
                .lazy_names
                .contains_key(&(module.to_owned(), name.to_owned()))
        {
            return Ok(());
        }
        Err(Box::new(
            Diagnostic::new(
                DiagnosticCode::error(201),
                span,
                format!(
                    "using `{module}.{name}`, which `{module}` imports lazily, from another module \
                     is planned for TyPy 0.4 (MOD-11)"
                ),
            )
            .with_rule(RuleId::MOD_11)
            .with_help(format!(
                "import it eagerly in `{module}`, or import it from its own module"
            )),
        ))
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
            "sys" => Some(("0.3", RuleId::LIB_04)),
            "math" => Some(("0.3", RuleId::LIB_05)),
            "dataclasses" | "typing" | "collections" => Some(("0.4", RuleId::LIB_06)),
            "ctypes" => Some(("0.5", RuleId::FFI_01)),
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
            .with_help("TyPy 0.3 provides `sys`, `math`, `typing`, `collections.abc` and `types`"),
        }))
    }
}

/// Returns the error for a name of `typing`, `collections.abc` or `types` that TyPy 0.3 does
/// not give: the rest of `typing` and `collections.abc` come in 0.4 (LIB-06).
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
    Diagnostic::new(
        DiagnosticCode::error(201),
        alias.name.span,
        format!("`{module}.{name}` is planned for TyPy 0.4 (LIB-06)"),
    )
    .with_rule(RuleId::LIB_06)
    .with_help("TyPy 0.3 gives `Iterator` and `Iterable` for annotations")
}

/// Returns the error for a relative import that names no module, with CPython's message.
fn imports_nothing(span: Span, message: &str) -> Diagnostic {
    Diagnostic::new(DiagnosticCode::error(206), span, message).with_rule(RuleId::MOD_02)
}

/// Returns E0426 for an import of a module that is checked after this one: an import inside
/// a function of a module that the imported module imports at module level.
fn checked_later(span: Span, module: &ModuleName) -> Diagnostic {
    Diagnostic::new(
        DiagnosticCode::error(426),
        span,
        format!(
            "`{module}` imports this module at module level, so importing it inside this function is planned for TyPy 0.4"
        ),
    )
    .with_rule(RuleId::MOD_05)
    .with_help("import it at module level in the other direction, or move the shared code to a third module")
}

impl BodyChecker<'_> {
    /// Checks an import statement: each module it initialises, and each name it binds to a
    /// module or to a copy of a module variable's value, typed as an assignment is.
    pub(super) fn import_statement(&mut self, statement: &Stmt, span: Span) -> Option<TypedStmt> {
        let resolved = match self.program.resolve_import(statement) {
            Ok(resolved) => resolved,
            Err(diagnostic) => {
                self.diagnostics.push(*diagnostic);
                return None;
            }
        };
        let mut bindings = Vec::new();
        let mut lazy_callables = Vec::new();
        for (name, imported, name_span) in resolved.names {
            let ty = match &imported {
                Imported::Function(_) | Imported::Class(_) => {
                    lazy_callables.push(name);
                    continue;
                }
                Imported::Annotation(_) | Imported::Standard(_) => continue,
                Imported::Variable { source, .. } => {
                    self.program.symbols[source.0 as usize].ty.clone()
                }
                Imported::Module(module) => Type::Module(module.as_str().to_owned()),
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
                Imported::Function(_)
                | Imported::Class(_)
                | Imported::Annotation(_)
                | Imported::Standard(_) => None,
            };
            bindings.extend(self.assign(&target, ty, &value).and_then(binding));
        }
        let import = TypedStmt::Import {
            modules: resolved.modules,
            bindings,
            span,
        };
        if self.in_function() || !typy_source::is_lazy(statement, &self.program.lazy_modules) {
            return Some(import);
        }
        self.lazy_import(import, lazy_callables, span)
    }

    /// Makes a lazy import of an import statement. Each name it binds gets its own function,
    /// which runs the import for that name alone, so rebinding one name cancels only its own
    /// import, as in CPython. A name bound by two lazy imports is refused, since its first
    /// use would have to know which of them ran last.
    fn lazy_import(
        &mut self,
        import: TypedStmt,
        callables: Vec<String>,
        span: Span,
    ) -> Option<TypedStmt> {
        let TypedStmt::Import {
            modules, bindings, ..
        } = import
        else {
            return None;
        };
        let mut names: Vec<(String, Option<ImportBinding>)> = bindings
            .into_iter()
            .map(|binding| {
                let (ImportBinding::Module { target, .. } | ImportBinding::Value { target, .. }) =
                    &binding;
                (
                    self.program.symbols[target.0 as usize].name.clone(),
                    Some(binding),
                )
            })
            .collect();
        names.extend(callables.into_iter().map(|name| (name, None)));
        let first = self.program.next_function().0;
        let module = self.program.module.clone();
        let twice = names.iter().zip(first..).find(|((name, _), id)| {
            self.program
                .lazy_names
                .get(&(module.clone(), name.clone()))
                .is_some_and(|known| known.0 != *id)
        });
        if let Some(((name, _), _)) = twice {
            self.diagnostics.push(
                Diagnostic::new(
                    DiagnosticCode::error(201),
                    span,
                    format!(
                        "binding `{name}` by a second lazy import is planned for TyPy 0.4 (MOD-11)"
                    ),
                )
                .with_rule(RuleId::MOD_11),
            );
            return None;
        }
        let mut reifies = Vec::new();
        let mut targets = Vec::new();
        for (name, binding) in names {
            let target = binding.as_ref().map(
                |(ImportBinding::Module { target, .. } | ImportBinding::Value { target, .. })| {
                    *target
                },
            );
            targets.extend(target);
            let import = TypedStmt::Import {
                modules: modules.clone(),
                bindings: binding.into_iter().collect(),
                span,
            };
            reifies.push(self.program.add_lazy_import(name, import, target, span));
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
            if matches!(imported, Imported::Variable { .. } | Imported::Module(_)) {
                self.bind_local(&Expr::Name { id: name, span });
            }
        }
    }

    /// Reads `module.name` through a module object: a module variable, read as it is now; a
    /// class, as a type object; or a submodule (MOD-10).
    pub(super) fn module_attribute(&mut self, module: &str, name: &str, span: Span) -> TypedExpr {
        if let Err(diagnostic) = self.program.check_not_lazy(module, name, span) {
            return self.error(*diagnostic, span);
        }
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
        let child = format!("{module}.{name}");
        if self.program.loaded.contains(&child) {
            return Self::typed(
                TypedExprKind::Submodule(child.clone()),
                Type::Module(child),
                span,
            );
        }
        let (code, rule, message) = if let Named::Function(_) = found {
            (
                201,
                RuleId::FUN_05,
                format!("using `{module}.{name}` as a value is planned for TyPy 1.0 (FUN-05)"),
            )
        } else {
            (
                425,
                RuleId::SMT_07,
                format!("module `{module}` has no name `{name}`"),
            )
        };
        self.error(
            Diagnostic::new(DiagnosticCode::error(code), span, message).with_rule(rule),
            span,
        )
    }

    /// Returns the type object of a class as a value, an exception class's for one.
    pub(super) fn class_object(&self, class: ClassId, name: &str, span: Span) -> TypedExpr {
        let of = match self.program.exceptions.get(&class) {
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
        if let Err(diagnostic) = self.program.check_not_lazy(module, &name.name, name.span) {
            return self.error(*diagnostic, span);
        }
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
            Some(Named::Variable(_) | Named::Nothing) => {}
        }
        for argument in arguments {
            self.expression(argument);
        }
        let read = self.module_attribute(module, &name.name, name.span);
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
                // A function's own import of a name hides the module's lazy import of it.
                let key = (self.program.module.clone(), name.clone());
                if let Some(lazy) = self.program.lazy_names.remove(&key) {
                    saved.lazy.push((name.clone(), lazy));
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
                    Imported::Variable { .. } | Imported::Module(_) => {}
                }
            }
        }
        saved
    }

    /// Puts back the names `import_names_of` replaced, newest first.
    pub(super) fn restore_names(&mut self, saved: SavedNames) {
        for (name, lazy) in saved.lazy {
            let key = (self.program.module.clone(), name);
            self.program.lazy_names.insert(key, lazy);
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
            Stmt::Try { .. } => {
                for block in statement.try_blocks() {
                    imports_in(block, found);
                }
            }
            _ => {}
        }
    }
}
