//! The program being built while bodies are checked: symbols, finished functions and what every body may refer to.

use std::collections::{BTreeMap, BTreeSet};

use typy_diagnostics::Span;

use crate::check::{ClassInfo, Definition};
use crate::exception::ExceptionClass;
use crate::typed::{
    Attribute, ClassId, FunctionId, FunctionSignature, Symbol, SymbolId, SymbolKind, TypedClass,
    TypedFunction, TypedModule, TypedProgram, TypedStmt,
};
use crate::types::Type;

/// The program being built: symbols, finished functions, and what every body may refer to.
#[derive(Clone)]
pub(crate) struct Program {
    pub(super) module: String,
    pub(super) symbols: Vec<Symbol>,
    pub(super) functions: Vec<Option<TypedFunction>>,
    pub(super) function_ids: BTreeMap<String, FunctionId>,
    pub(super) signatures: Vec<FunctionSignature>,
    /// The captures of each nested function (FUN-04).
    pub(super) closures: BTreeMap<FunctionId, Vec<crate::typed::CapturedVariable>>,
    /// The variables of each function that live in a cell (FUN-04).
    pub(super) cells: BTreeMap<FunctionId, BTreeSet<crate::typed::SymbolId>>,
    /// The variables nested functions declare `nonlocal`, with their captured cells (SMT-09).
    pub(super) nonlocals: BTreeMap<crate::typed::SymbolId, u32>,
    pub(super) module_variables: BTreeSet<String>,
    /// The symbol of each module variable, made before any body is checked (SMT-08).
    pub(super) module_symbols: BTreeMap<String, SymbolId>,
    /// The module variables some function declares `global`, which a test cannot narrow,
    /// because a call may assign them.
    pub(super) assigned_in_functions: BTreeSet<String>,
    /// Each variable declared `Final`, with the name its declaration assigns, which alone may
    /// assign it (TYP-08).
    pub(super) final_variables: BTreeMap<SymbolId, Span>,
    /// Whether the module variables have only the types a first look at the module body gave
    /// them, while the `__init__` methods are checked.
    pub(super) provisional: bool,
    pub(super) classes: Vec<ClassInfo>,
    /// The module of each class, indexed by `ClassId`.
    pub(super) class_modules: Vec<String>,
    pub(super) class_ids: BTreeMap<String, ClassId>,
    /// The classes that derive from an exception class (CLS-03).
    pub(super) instances: BTreeMap<ClassId, Type>,
    /// The modules checked so far, in the order they were checked.
    pub(super) modules: Vec<TypedModule>,
    /// The names each module checked so far defines, by the module's name (ADR-0015).
    pub(super) scopes: BTreeMap<String, ModuleScope>,
    /// The modules of the program's files, and the standard library modules it imports.
    pub(super) loaded: BTreeSet<String>,
    pub(super) standard: BTreeSet<String>,
    /// The package a relative import of the module being checked starts from (MOD-02).
    pub(super) package: Option<typy_source::ModuleName>,
    /// The path of the module being checked, from the project root.
    pub(super) path: String,
    /// For each name the module binds to a module at module level, the classes of that
    /// module, which annotations such as `shapes.Point` name.
    pub(super) module_classes: BTreeMap<String, BTreeMap<String, ClassId>>,
    /// For each name bound to a module of the program, that module's type aliases (SMT-10).
    pub(super) module_aliases: BTreeMap<String, crate::check::AliasTable>,
    /// The type aliases of other modules the module, or the function being checked, imports
    /// by name (SMT-12).
    pub(super) imported_aliases: crate::check::AliasTable,
    /// The module's own type aliases as other modules read them, kept when the module ends.
    pub(super) exported_aliases: crate::check::AliasTable,
    /// The type aliases read as values, by module and name (SMT-12).
    type_alias_objects: Vec<crate::typed::TypedTypeAlias>,
    /// The type aliases of the module being checked, which only its annotations read
    /// (SMT-10).
    pub(super) type_aliases: BTreeSet<String>,
    /// The type each type parameter stands for while a generic body or a copy of it is
    /// checked (GEN-01).
    pub(crate) type_arguments: BTreeMap<String, Type>,
    /// The generic functions of the program and the copies made of them (GEN-03).
    pub(crate) generics: super::generic_copy::Generics,
    /// The classes that satisfy each protocol, and the dispatchers of protocol values
    /// (GEN-10, ADR-0041).
    pub(crate) protocol_values: super::protocol_value::ProtocolValues,
    /// The generic classes of the program and the copies made of them, which annotations,
    /// read where only a shared reference is at hand, may add to (ADR-0028).
    pub(crate) class_copies: std::cell::RefCell<super::class_copy::ClassCopies>,
    /// The names of `typing`, `collections.abc` and `types` the module imports, and the
    /// names bound to those modules, which annotations read (TYP-15).
    pub(super) annotation_names: BTreeMap<String, crate::check::AnnotationName>,
    pub(super) annotation_modules: BTreeMap<String, String>,
    /// The functions of built-in modules the module imports by name, each under its name
    /// (ADR-0018).
    pub(super) module_functions: BTreeMap<String, super::builtin_module::ModuleFunction>,
    /// The built-in modules the program imports, whose module objects it holds.
    pub(super) builtin_modules: BTreeSet<String>,
    /// Whether the program closes a text file, by `close()` or a `with` block (LIB-14).
    pub(super) closes_text_files: bool,
    /// Whether the program can make a `str` with a surrogate standard output cannot write
    /// (LEX-10).
    pub(super) makes_surrogates: bool,
    /// The minor version of the program's Python floor, such as 12 for 3.12.
    pub(super) floor: u8,
    /// The operating system the program is built for, which `sys.platform` names.
    pub(super) platform: typy_source::Platform,
    /// The modules the module's `__lazy_modules__` lists, whose imports are lazy (MOD-07).
    pub(super) lazy_modules: BTreeSet<String>,
    /// Each function that runs a lazy import for one name, with the variable it binds; a
    /// function or a class imported lazily binds none (MOD-06).
    pub(super) lazy_imports: BTreeMap<FunctionId, Option<SymbolId>>,
    /// Each name a module binds by a lazy import, with the module, and the function that runs
    /// the import for it. Its IDs are the same in the first look and in the final check of a
    /// module body, since only module bodies add functions while checking.
    pub(super) lazy_names: BTreeMap<(String, String), FunctionId>,
    /// The module variables some other module assigns through its module object, such as
    /// `settings.debug = True`, which a test cannot narrow either.
    pub(super) assigned_elsewhere: BTreeSet<(String, String)>,
    /// The creations of exceptions this check records (EXC-15, ADR-0034).
    pub(super) creations: crate::exception_arguments::Creations,
    /// The creations the checks before this one recorded, which type `args`; none in a first
    /// check.
    pub(super) known_creations: Option<std::rc::Rc<crate::exception_arguments::Creations>>,
    /// Where a body first reads `args` as a value, if one does.
    pub(super) arguments_read: Option<Span>,
    /// The names of the module being checked that imports bind to several functions or
    /// classes, or inside a block, with their binding variables (MOD-12, ADR-0036).
    pub(super) rebound: BTreeMap<String, super::rebound::Rebound>,
    /// The function of each lazy import that binds such a name, by where it names it.
    pub(super) rebound_reify: BTreeMap<Span, FunctionId>,
    /// The names each module checked so far reads, kept for bodies of that module checked
    /// later (MOD-13, ADR-0037).
    pub(super) contexts: BTreeMap<String, super::ModuleContext>,
    /// The `ctypes` declarations, libraries and functions (FFI-01 to FFI-03).
    pub(crate) foreign: super::foreign_call::Foreign,
    /// The class variables the program assigns through a subclass that inherits them
    /// (ADR-0037).
    pub(super) subclass_variables: crate::check::subclass_variables::SubclassVariables,
    /// The classes of every module that some class derives from, by module and name, read
    /// before any module is checked, so a subclass in a module checked later counts
    /// (ADR-0084).
    pub(super) derived_classes: BTreeSet<crate::check::class_index::ClassKey>,
    /// The hidden function of each built-in function used as a value, by its module, its
    /// name and the parameter types of its `Callable` (FUN-08, ADR-0077).
    builtin_values: Vec<(String, String, Vec<Type>, FunctionId)>,
    /// The operator and comparison methods that return `T | NotImplementedType` (CLS-21).
    pub(crate) giving_way: crate::check::not_implemented::GivingWay,
}

/// What a first look at a module body may change, which `keep_module_types` undoes.
pub(crate) struct Snapshot {
    symbols: usize,
    functions: usize,
    signatures: usize,
    classes: Vec<ClassInfo>,
    first_class: usize,
    /// The generic copies made before the first look, whose IDs the look's copies follow.
    generics: super::generic_copy::Generics,
    class_copies: super::class_copy::ClassCopies,
    /// The fits of protocols before the look, whose copies' IDs another copy may take next.
    protocol_values: super::protocol_value::ProtocolValues,
}

/// The names one module defines, which other modules import.
#[derive(Clone, Debug, Default)]
pub(crate) struct ModuleScope {
    pub(super) function_ids: BTreeMap<String, FunctionId>,
    pub(super) class_ids: BTreeMap<String, ClassId>,
    pub(super) module_symbols: BTreeMap<String, SymbolId>,
    pub(super) type_aliases: BTreeSet<String>,
    /// The module's type aliases as other modules read them (SMT-12).
    pub(super) alias_values: crate::check::AliasTable,
    /// The names imports bind to more than one function or class, which another module
    /// cannot read yet (MOD-14).
    pub(super) rebound: BTreeSet<String>,
}

impl Program {
    /// Returns a program with no module checked yet.
    pub(crate) fn empty() -> Self {
        Self {
            module: String::new(),
            symbols: Vec::new(),
            functions: Vec::new(),
            function_ids: BTreeMap::new(),
            signatures: Vec::new(),
            closures: BTreeMap::new(),
            cells: BTreeMap::new(),
            nonlocals: BTreeMap::new(),
            module_variables: BTreeSet::new(),
            module_symbols: BTreeMap::new(),
            assigned_in_functions: BTreeSet::new(),
            final_variables: BTreeMap::new(),
            provisional: false,
            classes: Vec::new(),
            class_modules: Vec::new(),
            class_ids: BTreeMap::new(),
            instances: BTreeMap::new(),
            modules: Vec::new(),
            scopes: BTreeMap::new(),
            loaded: BTreeSet::new(),
            standard: BTreeSet::new(),
            package: None,
            path: String::new(),
            module_classes: BTreeMap::new(),
            module_aliases: BTreeMap::new(),
            imported_aliases: BTreeMap::new(),
            exported_aliases: BTreeMap::new(),
            type_alias_objects: Vec::new(),
            type_aliases: BTreeSet::new(),
            type_arguments: BTreeMap::new(),
            generics: super::generic_copy::Generics::default(),
            protocol_values: super::protocol_value::ProtocolValues::default(),
            class_copies: std::cell::RefCell::default(),
            annotation_names: BTreeMap::new(),
            annotation_modules: BTreeMap::new(),
            module_functions: BTreeMap::new(),
            builtin_modules: BTreeSet::new(),
            closes_text_files: false,
            makes_surrogates: false,
            floor: 12,
            platform: typy_source::Platform::host(),
            lazy_modules: BTreeSet::new(),
            lazy_imports: BTreeMap::new(),
            lazy_names: BTreeMap::new(),
            assigned_elsewhere: BTreeSet::new(),
            creations: crate::exception_arguments::Creations::default(),
            known_creations: None,
            arguments_read: None,
            rebound: BTreeMap::new(),
            rebound_reify: BTreeMap::new(),
            contexts: BTreeMap::new(),
            foreign: super::foreign_call::Foreign::default(),
            subclass_variables: crate::check::subclass_variables::SubclassVariables::default(),
            derived_classes: BTreeSet::new(),
            builtin_values: Vec::new(),
            giving_way: crate::check::not_implemented::GivingWay::new(),
        }
    }

    /// Returns the modules the module's `__lazy_modules__` lists, whose imports are lazy.
    pub(crate) const fn lazy_modules(&self) -> &BTreeSet<String> {
        &self.lazy_modules
    }

    /// Notes the modules of the program and the standard library modules it imports, and
    /// the module variables assigned through module objects, before any module is checked.
    pub(crate) fn set_project(
        &mut self,
        loaded: BTreeSet<String>,
        standard: BTreeSet<String>,
        assigned_elsewhere: BTreeSet<(String, String)>,
    ) {
        self.loaded = loaded;
        self.standard = standard;
        self.assigned_elsewhere = assigned_elsewhere;
    }

    /// Returns the name of the module being checked.
    pub(crate) fn module_name(&self) -> &str {
        &self.module
    }

    /// Returns the names the module `module` read when it was checked.
    pub(crate) fn context_of(&self, module: &str) -> Option<super::ModuleContext> {
        self.contexts.get(module).cloned()
    }

    /// Returns how many classes the modules checked so far define, the first ID of the next.
    pub(crate) fn class_count(&self) -> usize {
        self.classes.len()
    }

    /// Returns how many functions the modules checked so far define, the first ID of the next.
    pub(crate) fn function_count(&self) -> usize {
        self.functions.len()
    }

    /// Starts checking a module: its functions and classes follow those of the modules
    /// before it, and its names replace theirs in the tables bodies read.
    #[allow(
        clippy::too_many_arguments,
        reason = "the parts of a module the collection found"
    )]
    pub(crate) fn begin_module(
        &mut self,
        module: &typy_source::Module,
        path: &str,
        definitions: &[Definition<'_>],
        module_variables: BTreeSet<String>,
        classes: Vec<ClassInfo>,
        class_ids: BTreeMap<String, ClassId>,
        instances: BTreeMap<ClassId, Type>,
    ) {
        let first = self.functions.len();
        self.note_generics(first, definitions);
        // Methods are reached through their instances, so only module functions have names.
        self.function_ids = definitions
            .iter()
            .enumerate()
            .filter(|(_, definition)| definition.class.is_none())
            .map(|(index, definition)| (definition.name.clone(), FunctionId(id(first + index))))
            .collect();
        self.functions
            .extend(std::iter::repeat_n(None, definitions.len()));
        self.signatures.extend(
            definitions
                .iter()
                .map(|definition| definition.signature.clone()),
        );
        module.name.as_str().clone_into(&mut self.module);
        path.clone_into(&mut self.path);
        self.package = module.package();
        self.module_variables = module_variables;
        self.module_symbols = BTreeMap::new();
        self.assigned_in_functions = self
            .assigned_elsewhere
            .iter()
            .filter(|(owner, _)| owner == module.name.as_str())
            .map(|(_, name)| name.clone())
            .collect();
        self.module_classes = BTreeMap::new();
        self.module_aliases = BTreeMap::new();
        self.imported_aliases = BTreeMap::new();
        self.class_modules
            .extend(std::iter::repeat_n(self.module.clone(), classes.len()));
        let first_class = self.classes.len();
        self.classes.extend(classes);
        self.class_ids = class_ids;
        self.instances.extend(instances);
        // A generic class's copies are made from its definitions, which have no bridges
        // (CLS-30); a protocol has no instances, and a value of it reaches the methods of the
        // object's class (ADR-0041).
        for index in first_class..self.classes.len() {
            let class = ClassId(id(index));
            if self.is_protocol(class) {
                self.make_protocol_dispatchers(class);
                continue;
            }
            if !self.class_copies.borrow().roots.contains_key(&class) {
                self.add_class_method_bridges(class);
                self.add_comparison_bridges(class);
                self.add_truth_bridge(class);
                self.add_override_adapters(class);
                self.add_static_bridges(class);
            }
        }
    }

    /// Notes the module about to be checked, whose imports resolve against its package.
    pub(crate) fn prepare_module(&mut self, module: &typy_source::Module) {
        self.type_aliases = module
            .syntax
            .body
            .iter()
            .filter_map(|statement| match statement {
                typy_source::syntax::Stmt::TypeAlias { name, .. } => Some(name.name.clone()),
                _ => None,
            })
            .collect();
        self.package = module.package();
        module.name.as_str().clone_into(&mut self.module);
        self.lazy_modules = typy_source::lazy_modules(&module.syntax.body);
        self.module_functions = BTreeMap::new();
    }

    /// Sets the minor version of the program's Python floor.
    pub(crate) const fn set_floor(&mut self, minor: u8) {
        self.floor = minor;
    }

    /// Sets the operating system the program is built for.
    pub(crate) const fn set_platform(&mut self, platform: typy_source::Platform) {
        self.platform = platform;
    }

    /// Adds the functions of built-in modules the module's imports bind by name.
    pub(crate) fn import_module_functions(
        &mut self,
        functions: &BTreeMap<String, super::builtin_module::ModuleFunction>,
    ) {
        self.module_functions.extend(
            functions
                .iter()
                .map(|(name, function)| (name.clone(), *function)),
        );
    }

    /// Returns the ID the next function added while checking a module body takes.
    /// Returns the hidden function of the module being checked that calls the built-in
    /// `name` with arguments of `parameters`, when one was made.
    pub(super) fn builtin_value(&self, name: &str, parameters: &[Type]) -> Option<FunctionId> {
        self.builtin_values
            .iter()
            .find(|(module, each, types, _)| {
                *module == self.module && each == name && types.as_slice() == parameters
            })
            .map(|(.., function)| *function)
    }

    /// Keeps `function`, the hidden function that calls the built-in `name` with arguments of
    /// `parameters`, for the module being checked.
    pub(super) fn add_builtin_value(
        &mut self,
        name: &str,
        parameters: Vec<Type>,
        function: FunctionId,
    ) {
        self.builtin_values
            .push((self.module.clone(), name.to_owned(), parameters, function));
    }

    pub(super) fn next_function(&self) -> FunctionId {
        FunctionId(id(self.functions.len()))
    }

    /// Adds the function that runs a lazy import for the name `name`, when its first use
    /// needs it: its body is the import, run eagerly, binding `target` if the name is a
    /// variable (MOD-06).
    pub(super) fn add_lazy_import(
        &mut self,
        name: String,
        import: TypedStmt,
        target: Option<SymbolId>,
        span: Span,
    ) -> FunctionId {
        let id = self.next_function();
        let signature = FunctionSignature {
            parameters: Vec::new(),
            returns: Type::NoneType,
        };
        self.signatures.push(signature.clone());
        self.functions.push(Some(TypedFunction {
            id,
            name: "<lazy import>".to_owned(),
            qualified_name: format!("<lazy import {}>", id.0),
            module: self.module.clone(),
            signature,
            parameters: Vec::new(),
            locals: Vec::new(),
            body: vec![import],
            span,
            generic: false,
            named_targets: Vec::new(),
            generator: None,
        }));
        self.lazy_imports.insert(id, target);
        // A name a later lazy import binds again keeps its first function, which lowering
        // reads together with the others that bind the same variable (MOD-11).
        self.lazy_names
            .entry((self.module.clone(), name))
            .or_insert(id);
        id
    }

    /// Returns, for each name bound to a module, that module's classes.
    pub(crate) fn module_classes_of(
        &self,
        modules: &BTreeMap<String, String>,
    ) -> BTreeMap<String, BTreeMap<String, ClassId>> {
        modules
            .iter()
            .filter_map(|(name, module)| {
                let scope = self.scopes.get(module)?;
                Some((name.clone(), scope.class_ids.clone()))
            })
            .collect()
    }

    /// Returns, for each name bound to a module of the program, that module's type aliases.
    pub(crate) fn module_aliases_of(
        &self,
        modules: &BTreeMap<String, String>,
    ) -> BTreeMap<String, crate::check::AliasTable> {
        modules
            .iter()
            .filter_map(|(name, module)| {
                let scope = self.scopes.get(module)?;
                Some((name.clone(), scope.alias_values.clone()))
            })
            .collect()
    }

    /// Returns the index of the object of the type alias `name` of `module`, adding it on
    /// first use (SMT-12).
    pub(super) fn type_alias_object(&mut self, module: &str, name: &str) -> u32 {
        let found = self
            .type_alias_objects
            .iter()
            .position(|alias| alias.module == module && alias.name == name)
            .unwrap_or_else(|| {
                self.type_alias_objects.push(crate::typed::TypedTypeAlias {
                    name: name.to_owned(),
                    module: module.to_owned(),
                });
                self.type_alias_objects.len() - 1
            });
        id(found)
    }

    /// Keeps the module's type aliases, with what its annotations see, for the modules that
    /// import them (SMT-12).
    pub(crate) fn export_aliases(
        &mut self,
        aliases: &BTreeMap<String, crate::check::type_alias::Alias>,
        classes: &BTreeMap<String, ClassId>,
        names: &crate::check::ImportedNames,
    ) {
        self.exported_aliases = aliases
            .iter()
            .map(|(name, alias)| {
                (
                    name.clone(),
                    std::rc::Rc::new(crate::check::ExportedAlias {
                        module: self.module.clone(),
                        name: name.clone(),
                        parameters: alias.parameters.clone(),
                        value: alias.value.clone(),
                        classes: classes.clone(),
                        names: names.clone(),
                    }),
                )
            })
            .collect();
    }

    /// Returns the exception classes of the modules checked so far.
    pub(crate) fn known_instances(&self) -> &BTreeMap<ClassId, Type> {
        &self.instances
    }

    /// Returns the signatures of the functions of the modules checked so far, indexed by ID.
    pub(crate) fn signatures(&self) -> &[FunctionSignature] {
        &self.signatures
    }

    /// Returns the classes of the modules checked so far, indexed by ID.
    pub(crate) fn classes(&self) -> &[ClassInfo] {
        &self.classes
    }

    /// Returns the class of an exception class, or `None` for any other class.
    pub(super) fn exception(&self, class: ClassId) -> Option<&ExceptionClass> {
        match self.instances.get(&class) {
            Some(Type::Exception(exception)) => Some(exception),
            _ => None,
        }
    }

    /// Adds the functions a module imports by name to its table of functions, and the
    /// classes of the modules its names bind, for annotations.
    pub(crate) fn import_names(
        &mut self,
        functions: &BTreeMap<String, FunctionId>,
        annotations: crate::check::ImportedNames,
    ) {
        self.function_ids
            .extend(functions.iter().map(|(name, id)| (name.clone(), *id)));
        self.module_classes = annotations.modules;
        self.module_aliases = annotations.module_aliases;
        self.imported_aliases = annotations.aliases;
        self.annotation_names = annotations.names;
        self.annotation_modules = annotations.standard;
    }

    /// Ends checking a module whose body is the function `body`, and keeps its names for
    /// the modules that import it.
    pub(crate) fn end_module(&mut self, body: FunctionId) {
        let context = self.module_context();
        self.contexts.insert(self.module.clone(), context);
        self.modules.push(TypedModule {
            name: self.module.clone(),
            path: self.path.clone(),
            body,
        });
        self.scopes.insert(
            self.module.clone(),
            ModuleScope {
                function_ids: self.function_ids.clone(),
                class_ids: self.class_ids.clone(),
                module_symbols: self.module_symbols.clone(),
                type_aliases: std::mem::take(&mut self.type_aliases),
                alias_values: std::mem::take(&mut self.exported_aliases),
                rebound: self
                    .rebound
                    .iter()
                    .filter(|(_, rebound)| rebound.several())
                    .map(|(name, _)| name.clone())
                    .collect(),
            },
        );
    }

    /// Returns the error for `module.name` read from another module, when imports bind the
    /// name to more than one function or class there.
    pub(super) fn rebound_elsewhere(
        &self,
        module: &str,
        name: &str,
        span: Span,
    ) -> Option<typy_diagnostics::Diagnostic> {
        self.scopes
            .get(module)
            .is_some_and(|scope| scope.rebound.contains(name))
            .then(|| super::rebound::rebound_planned(name, "used from another module", span))
    }

    /// Returns the type of the instances of a class: an exception for an exception class.
    pub(super) fn instance_type(&self, class: ClassId) -> Type {
        self.instances.get(&class).cloned().unwrap_or_else(|| {
            Type::instance(crate::class_type::InstanceClass::root(
                class,
                &self.classes[class.0 as usize].name,
            ))
        })
    }

    /// Returns true when the class is a protocol, which has no instances (GEN-04).
    pub(crate) fn is_protocol(&self, class: ClassId) -> bool {
        matches!(self.instances.get(&class), Some(Type::Instance(instance)) if instance.protocol)
    }

    /// Returns the method `name` of a class, its own or the nearest base's.
    pub(super) fn method(&self, class: ClassId, name: &str) -> Option<FunctionId> {
        // A method of a protocol is reached through its dispatcher (ADR-0041).
        if self.is_protocol(class) && self.classes[class.0 as usize].method(name).is_some() {
            return self.dispatcher(class, name);
        }
        let mut current = Some(class);
        while let Some(class) = current {
            let info = &self.classes[class.0 as usize];
            if let Some(method) = info.method(name) {
                return Some(method);
            }
            current = info.base;
        }
        None
    }

    /// Returns each attribute of `class` with a default, its own or a base's, with the module
    /// variable that holds the default (ADR-0024).
    fn default_variables(&self, class: ClassId) -> Vec<(usize, SymbolId)> {
        let mut found = Vec::new();
        let mut current = Some(class);
        while let Some(each) = current {
            let info = &self.classes[each.0 as usize];
            let module = &self.class_modules[each.0 as usize];
            for index in &info.defaults {
                // The nearest class's default is the one an instance holds.
                if found.iter().any(|(each, _)| each == index) {
                    continue;
                }
                let name =
                    crate::check::class_value_variable(&info.name, &info.attributes[*index].name);
                let symbol = self
                    .scopes
                    .get(module)
                    .and_then(|scope| scope.module_symbols.get(&name))
                    .copied();
                found.extend(symbol.map(|symbol| (*index, symbol)));
            }
            current = info.base;
        }
        found
    }

    /// Returns the class whose own attribute is the attribute at `index` of `class`: the base
    /// that declares an inherited one, whose check gives it its type.
    pub(super) fn declaring(&self, class: ClassId, index: usize) -> ClassId {
        let mut current = class;
        loop {
            let info = &self.classes[current.0 as usize];
            match info.base {
                Some(base) if index < info.inherited => current = base,
                _ => return current,
            }
        }
    }

    /// Returns the type of the attribute at `index` of `class`, once it is known.
    pub(super) fn attribute_ty(&self, class: ClassId, index: usize) -> Option<Type> {
        let owner = self.declaring(class, index);
        let known = self.classes[owner.0 as usize].attributes[index].ty.clone();
        // A copy of a generic class has the generic class's attribute types, with its type
        // arguments, before its own `__init__` is checked (ADR-0028).
        known.or_else(|| self.copied_attribute_ty(owner, index))
    }

    /// Makes the symbol of each module variable, before any body is checked, and notes those
    /// a function declares `global`.
    pub(crate) fn declare_module_variables(
        &mut self,
        variables: &BTreeMap<String, Span>,
        definitions: &[Definition<'_>],
    ) {
        for (name, span) in variables {
            let symbol = self.add_symbol(name, SymbolKind::ModuleVariable, Type::Error, *span);
            self.module_symbols.insert(name.clone(), symbol);
            // A class variable, `Class.name`, may be assigned by any function, so a test of it
            // narrows nothing (ADR-0024).
            if name.contains('.') {
                self.assigned_in_functions.insert(name.clone());
            }
        }
        for definition in definitions {
            for name in crate::check::declared_globals(definition.body) {
                self.assigned_in_functions.insert(name.name.clone());
            }
        }
    }

    /// Returns what checking the module body changes, so `keep_module_types` can undo it:
    /// how many symbols, functions and signatures there are, and the module's own classes,
    /// numbered from `first_class`.
    pub(crate) fn snapshot(&self, first_class: usize) -> Snapshot {
        Snapshot {
            symbols: self.symbols.len(),
            functions: self.functions.len(),
            signatures: self.signatures.len(),
            classes: self.classes[first_class..].to_vec(),
            first_class,
            generics: self.generics.clone(),
            class_copies: self.class_copies.borrow().clone(),
            protocol_values: self.protocol_values.clone(),
        }
    }

    /// Undoes a first look at the module body but the types it gave the module variables,
    /// which the `__init__` methods read until the module body is checked again.
    pub(crate) fn keep_module_types(&mut self, snapshot: Snapshot) {
        let found: Vec<Type> = self
            .module_symbols
            .values()
            .map(|symbol| self.symbols[symbol.0 as usize].ty.clone())
            .collect();
        self.symbols.truncate(snapshot.symbols);
        let first_new = FunctionId(id(snapshot.functions));
        self.lazy_imports
            .retain(|function, _| *function < first_new);
        // The hidden functions of built-ins the look made are made again (FUN-08).
        self.builtin_values
            .retain(|(.., function)| *function < first_new);
        self.functions.truncate(snapshot.functions);
        self.signatures.truncate(snapshot.signatures);
        self.classes.truncate(snapshot.first_class);
        self.classes.extend(snapshot.classes);
        // The copies the look made took IDs it gives back; the module body makes them again.
        self.generics = snapshot.generics;
        self.class_copies.replace(snapshot.class_copies);
        self.protocol_values = snapshot.protocol_values;
        crate::protocol_fits::reset(&self.protocol_values.fits);
        crate::iterator_fits::reset(&self.protocol_values.iterations);
        let symbols: Vec<SymbolId> = self.module_symbols.values().copied().collect();
        for (symbol, ty) in symbols.into_iter().zip(found) {
            self.symbols[symbol.0 as usize].ty = ty;
        }
        self.provisional = true;
        // The variables now have their types, which the getters of class variables give.
        self.add_class_variable_getters();
    }

    /// Ends the first look: the module body is checked next, and gives the final types.
    pub(crate) fn end_first_look(&mut self) {
        self.provisional = false;
    }

    pub(super) fn add_symbol(
        &mut self,
        name: &str,
        kind: SymbolKind,
        ty: Type,
        span: Span,
    ) -> SymbolId {
        let symbol = SymbolId(id(self.symbols.len()));
        self.symbols.push(Symbol {
            id: symbol,
            name: name.to_owned(),
            kind,
            ty,
            span,
        });
        symbol
    }

    /// Returns each class as the typed program holds it.
    fn typed_classes(&self) -> Vec<TypedClass> {
        self.classes
            .iter()
            .enumerate()
            .map(|(index, class)| {
                let opaque = self.is_opaque_class(class.id);
                let generic = self
                    .class_copies
                    .borrow()
                    .copy(class.id)
                    .map(|copy| copy.root);
                TypedClass {
                    id: ClassId(id(index)),
                    name: class.name.clone(),
                    module: self.class_modules[index].clone(),
                    base: class.base,
                    exception: class.exception.clone(),
                    // An opaque class has no instances, so it holds nothing to lay out.
                    attributes: if opaque {
                        Vec::new()
                    } else {
                        class
                            .attributes
                            .iter()
                            .enumerate()
                            .map(|(position, attribute)| Attribute {
                                name: attribute.name.clone(),
                                ty: self
                                    .attribute_ty(class.id, position)
                                    .unwrap_or(crate::types::Type::Error),
                            })
                            .collect()
                    },
                    methods: if opaque {
                        Vec::new()
                    } else {
                        class.methods.clone()
                    },
                    init: if opaque {
                        None
                    } else {
                        self.method(class.id, "__init__")
                    },
                    defaults: self.default_variables(class.id),
                    adapters: if opaque {
                        Vec::new()
                    } else {
                        class.adapters.clone()
                    },
                    static_bridges: if opaque {
                        Vec::new()
                    } else {
                        class.static_bridges.clone()
                    },
                    span: class.span,
                    generic,
                    opaque,
                    steps: !opaque
                        && matches!(
                            self.protocol_values.iterations.get(&class.id),
                            Some(crate::iterator_fits::Iteration::Iterator(_))
                        ),
                    iterates: !opaque && self.protocol_values.iterations.contains_key(&class.id),
                }
            })
            .collect()
    }

    /// Returns the typed program, whose entry is the body of the module named `main`.
    pub(crate) fn finish(self, main: &str) -> TypedProgram {
        let entry = self
            .modules
            .iter()
            .find(|module| module.name == main)
            .map_or(FunctionId(0), |module| module.body);
        let classes = self.typed_classes();
        TypedProgram {
            symbols: self.symbols,
            functions: self.functions.into_iter().flatten().collect(),
            classes,
            modules: self.modules,
            lazy_imports: self.lazy_imports,
            builtin_modules: self.builtin_modules.into_iter().collect(),
            type_aliases: self.type_alias_objects,
            closes_text_files: self.closes_text_files,
            makes_surrogates: self.makes_surrogates
                || self.signatures.iter().any(|signature| {
                    signature.parameters.iter().any(|parameter| {
                        parameter
                            .default
                            .as_ref()
                            .is_some_and(crate::typed::TypedConstant::holds_unescapable_surrogate)
                    })
                }),
            entry,
            foreign_libraries: self.foreign.libraries,
            foreign_functions: self.foreign.functions,
            exports: None,
            library_version: None,
            warnings: typy_diagnostics::Diagnostics::new(),
            closures: self.closures,
            // A hidden function of a method is named `type.method` (ADR-0077).
            builtin_values: self
                .builtin_values
                .iter()
                .map(|(_, name, _, function)| {
                    let kind = if name.contains('.') {
                        crate::typed::BuiltinValue::Method
                    } else {
                        crate::typed::BuiltinValue::Function
                    };
                    (*function, kind)
                })
                .collect(),
            cells: self.cells,
            nonlocals: self.nonlocals,
            fits: (
                self.protocol_values.fits.clone(),
                self.protocol_values.iterations.clone(),
            ),
        }
    }
}

pub(super) fn id(index: usize) -> u32 {
    u32::try_from(index).unwrap_or(u32::MAX)
}
