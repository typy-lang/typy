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
        }
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
    pub(crate) fn begin_module(
        &mut self,
        module: &str,
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
        module.clone_into(&mut self.module);
        self.module_variables = module_variables;
        self.module_symbols = BTreeMap::new();
        self.assigned_in_functions = BTreeSet::new();
        self.class_modules
            .extend(std::iter::repeat_n(module.to_owned(), classes.len()));
        self.classes.extend(classes);
        self.class_ids = class_ids;
        self.exceptions.extend(exceptions);
    }

    /// Ends checking a module whose body is the function `body`.
    pub(crate) fn end_module(&mut self, body: FunctionId) {
        self.modules.push(TypedModule {
            name: self.module.clone(),
            body,
        });
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

    /// Gives each module variable the type it has in `other`, a copy of this program in which
    /// the module body was checked, while the `__init__` methods are checked.
    pub(crate) fn take_module_types(&mut self, other: &Self) {
        for symbol in self.module_symbols.values() {
            let index = symbol.0 as usize;
            self.symbols[index].ty = other.symbols[index].ty.clone();
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
