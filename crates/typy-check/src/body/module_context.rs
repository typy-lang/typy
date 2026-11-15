//! The names a module's bodies read while it is checked, set aside when the module ends so a
//! function body that imports a module checked after its own can be checked later in its own
//! module's names (MOD-13).

use std::collections::{BTreeMap, BTreeSet};

use super::Program;
use crate::typed::{ClassId, FunctionId, SymbolId};

/// The tables of the module being checked that its bodies read.
#[derive(Clone, Debug, Default)]
pub(crate) struct ModuleContext {
    module: String,
    path: String,
    package: Option<typy_source::ModuleName>,
    function_ids: BTreeMap<String, FunctionId>,
    class_ids: BTreeMap<String, ClassId>,
    module_variables: BTreeSet<String>,
    module_symbols: BTreeMap<String, SymbolId>,
    assigned_in_functions: BTreeSet<String>,
    module_classes: BTreeMap<String, BTreeMap<String, ClassId>>,
    module_aliases: BTreeMap<String, crate::check::AliasTable>,
    imported_aliases: crate::check::AliasTable,
    type_aliases: BTreeSet<String>,
    annotation_names: BTreeMap<String, crate::check::AnnotationName>,
    annotation_modules: BTreeMap<String, String>,
    module_functions: BTreeMap<String, super::builtin_module::ModuleFunction>,
    lazy_modules: BTreeSet<String>,
    rebound: BTreeMap<String, super::rebound::Rebound>,
    rebound_reify: BTreeMap<typy_diagnostics::Span, FunctionId>,
}

impl Program {
    /// Returns a copy of the tables of the module being checked.
    pub(crate) fn module_context(&self) -> ModuleContext {
        ModuleContext {
            module: self.module.clone(),
            path: self.path.clone(),
            package: self.package.clone(),
            function_ids: self.function_ids.clone(),
            class_ids: self.class_ids.clone(),
            module_variables: self.module_variables.clone(),
            module_symbols: self.module_symbols.clone(),
            assigned_in_functions: self.assigned_in_functions.clone(),
            module_classes: self.module_classes.clone(),
            module_aliases: self.module_aliases.clone(),
            imported_aliases: self.imported_aliases.clone(),
            type_aliases: self.type_aliases.clone(),
            annotation_names: self.annotation_names.clone(),
            annotation_modules: self.annotation_modules.clone(),
            module_functions: self.module_functions.clone(),
            lazy_modules: self.lazy_modules.clone(),
            rebound: self.rebound.clone(),
            rebound_reify: self.rebound_reify.clone(),
        }
    }

    /// Puts `context` in place of the tables of the module being checked, and returns them.
    pub(crate) fn swap_module_context(&mut self, mut context: ModuleContext) -> ModuleContext {
        std::mem::swap(&mut self.module, &mut context.module);
        std::mem::swap(&mut self.path, &mut context.path);
        std::mem::swap(&mut self.package, &mut context.package);
        std::mem::swap(&mut self.function_ids, &mut context.function_ids);
        std::mem::swap(&mut self.class_ids, &mut context.class_ids);
        std::mem::swap(&mut self.module_variables, &mut context.module_variables);
        std::mem::swap(&mut self.module_symbols, &mut context.module_symbols);
        std::mem::swap(
            &mut self.assigned_in_functions,
            &mut context.assigned_in_functions,
        );
        std::mem::swap(&mut self.module_classes, &mut context.module_classes);
        std::mem::swap(&mut self.module_aliases, &mut context.module_aliases);
        std::mem::swap(&mut self.imported_aliases, &mut context.imported_aliases);
        std::mem::swap(&mut self.type_aliases, &mut context.type_aliases);
        std::mem::swap(&mut self.annotation_names, &mut context.annotation_names);
        std::mem::swap(
            &mut self.annotation_modules,
            &mut context.annotation_modules,
        );
        std::mem::swap(&mut self.module_functions, &mut context.module_functions);
        std::mem::swap(&mut self.lazy_modules, &mut context.lazy_modules);
        std::mem::swap(&mut self.rebound, &mut context.rebound);
        std::mem::swap(&mut self.rebound_reify, &mut context.rebound_reify);
        context
    }

    /// Returns true when the import statement `statement` needs a module of the program that
    /// is not checked yet: one a module checked after this one imports at module level.
    pub(crate) fn imports_later_module(&self, statement: &typy_source::syntax::Stmt) -> bool {
        match self.resolve_import(statement) {
            Ok(resolved) => resolved.modules.iter().any(|module| {
                *module != self.module
                    && self.loaded.contains(module)
                    && !self.scopes.contains_key(module)
            }),
            Err(diagnostic) => diagnostic.code() == typy_diagnostics::DiagnosticCode::error(426),
        }
    }
}
