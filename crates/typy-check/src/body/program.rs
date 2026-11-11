//! The program being built while bodies are checked: symbols, finished functions and what every body may refer to.

use std::collections::{BTreeMap, BTreeSet};

use typy_diagnostics::Span;

use crate::check::{ClassInfo, Definition};
use crate::exception::ExceptionClass;
use crate::typed::{
    Attribute, ClassId, FunctionId, FunctionSignature, Symbol, SymbolId, SymbolKind, TypedClass,
    TypedFunction, TypedModule, TypedProgram,
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
    pub(super) module_variables: BTreeSet<String>,
    /// The symbol of each module variable, made before any body is checked (SMT-08).
    pub(super) module_symbols: BTreeMap<String, SymbolId>,
    /// The module variables some function declares `global`, which a test cannot narrow,
    /// because a call may assign them.
    pub(super) assigned_in_functions: BTreeSet<String>,
    /// Whether the module variables have only the types a first look at the module body gave
    /// them, while the `__init__` methods are checked.
    pub(super) provisional: bool,
    pub(super) classes: Vec<ClassInfo>,
    /// The module of each class, indexed by `ClassId`.
    pub(super) class_modules: Vec<String>,
    pub(super) class_ids: BTreeMap<String, ClassId>,
    /// The classes that derive from an exception class (CLS-03).
    pub(super) exceptions: BTreeMap<ClassId, ExceptionClass>,
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
    /// The module variables some other module assigns through its module object, such as
    /// `settings.debug = True`, which a test cannot narrow either.
    pub(super) assigned_elsewhere: BTreeSet<(String, String)>,
}

/// What a first look at a module body may change, which `keep_module_types` undoes.
pub(crate) struct Snapshot {
    symbols: usize,
    functions: usize,
    signatures: usize,
    classes: Vec<ClassInfo>,
    first_class: usize,
}

/// The names one module defines, which other modules import.
#[derive(Clone, Debug, Default)]
pub(crate) struct ModuleScope {
    pub(super) function_ids: BTreeMap<String, FunctionId>,
    pub(super) class_ids: BTreeMap<String, ClassId>,
    pub(super) module_symbols: BTreeMap<String, SymbolId>,
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
            module_variables: BTreeSet::new(),
            module_symbols: BTreeMap::new(),
            assigned_in_functions: BTreeSet::new(),
            provisional: false,
            classes: Vec::new(),
            class_modules: Vec::new(),
            class_ids: BTreeMap::new(),
            exceptions: BTreeMap::new(),
            modules: Vec::new(),
            scopes: BTreeMap::new(),
            loaded: BTreeSet::new(),
            standard: BTreeSet::new(),
            package: None,
            path: String::new(),
            module_classes: BTreeMap::new(),
            assigned_elsewhere: BTreeSet::new(),
        }
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
        exceptions: BTreeMap<ClassId, ExceptionClass>,
    ) {
        let first = self.functions.len();
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
        self.class_modules
            .extend(std::iter::repeat_n(self.module.clone(), classes.len()));
        self.classes.extend(classes);
        self.class_ids = class_ids;
        self.exceptions.extend(exceptions);
    }

    /// Notes the module about to be checked, whose imports resolve against its package.
    pub(crate) fn prepare_module(&mut self, module: &typy_source::Module) {
        self.package = module.package();
        module.name.as_str().clone_into(&mut self.module);
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

    /// Returns the exception classes of the modules checked so far.
    pub(crate) fn known_exceptions(&self) -> BTreeMap<ClassId, ExceptionClass> {
        self.exceptions.clone()
    }

    /// Adds the functions a module imports by name to its table of functions, and the
    /// classes of the modules its names bind, for annotations.
    pub(crate) fn import_names(
        &mut self,
        functions: &BTreeMap<String, FunctionId>,
        module_classes: BTreeMap<String, BTreeMap<String, ClassId>>,
    ) {
        self.function_ids
            .extend(functions.iter().map(|(name, id)| (name.clone(), *id)));
        self.module_classes = module_classes;
    }

    /// Ends checking a module whose body is the function `body`, and keeps its names for
    /// the modules that import it.
    pub(crate) fn end_module(&mut self, body: FunctionId) {
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
            },
        );
    }

    /// Returns the type of the instances of a class: an exception for an exception class.
    pub(super) fn instance_type(&self, class: ClassId) -> Type {
        self.exceptions.get(&class).cloned().map_or_else(
            || Type::Instance(class, self.classes[class.0 as usize].name.clone()),
            Type::Exception,
        )
    }

    /// Returns the method `name` of a class, its own or the nearest base's.
    pub(super) fn method(&self, class: ClassId, name: &str) -> Option<FunctionId> {
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
        self.classes[owner.0 as usize].attributes[index].ty.clone()
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
        self.functions.truncate(snapshot.functions);
        self.signatures.truncate(snapshot.signatures);
        self.classes.truncate(snapshot.first_class);
        self.classes.extend(snapshot.classes);
        let symbols: Vec<SymbolId> = self.module_symbols.values().copied().collect();
        for (symbol, ty) in symbols.into_iter().zip(found) {
            self.symbols[symbol.0 as usize].ty = ty;
        }
        self.provisional = true;
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

    /// Returns the typed program, whose entry is the body of the module named `main`.
    pub(crate) fn finish(self, main: &str) -> TypedProgram {
        let entry = self
            .modules
            .iter()
            .find(|module| module.name == main)
            .map_or(FunctionId(0), |module| module.body);
        let classes = self
            .classes
            .iter()
            .enumerate()
            .map(|(index, class)| TypedClass {
                id: ClassId(id(index)),
                name: class.name.clone(),
                module: self.class_modules[index].clone(),
                base: class.base,
                exception: class.exception.clone(),
                attributes: class
                    .attributes
                    .iter()
                    .enumerate()
                    .map(|(position, attribute)| Attribute {
                        name: attribute.name.clone(),
                        ty: self
                            .attribute_ty(class.id, position)
                            .unwrap_or(crate::types::Type::Error),
                    })
                    .collect(),
                methods: class.methods.clone(),
                init: self.method(class.id, "__init__"),
                span: class.span,
            })
            .collect();
        TypedProgram {
            symbols: self.symbols,
            functions: self.functions.into_iter().flatten().collect(),
            classes,
            modules: self.modules,
            entry,
        }
    }
}

pub(super) fn id(index: usize) -> u32 {
    u32::try_from(index).unwrap_or(u32::MAX)
}
