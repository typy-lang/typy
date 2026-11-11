//! The program being built while bodies are checked: symbols, finished functions and what every body may refer to.

use std::collections::{BTreeMap, BTreeSet};

use typy_diagnostics::Span;

use crate::check::{ClassInfo, Definition};
use crate::exception::ExceptionClass;
use crate::typed::{
    Attribute, ClassId, FunctionId, FunctionSignature, Symbol, SymbolId, SymbolKind, TypedClass,
    TypedFunction, TypedProgram,
};
use crate::types::Type;

/// The program being built: symbols, finished functions, and what every body may refer to.
pub(crate) struct Program {
    pub(super) module: String,
    pub(super) symbols: Vec<Symbol>,
    pub(super) functions: Vec<Option<TypedFunction>>,
    pub(super) function_ids: BTreeMap<String, FunctionId>,
    pub(super) signatures: Vec<FunctionSignature>,
    pub(super) module_variables: BTreeSet<String>,
    pub(super) classes: Vec<ClassInfo>,
    pub(super) class_ids: BTreeMap<String, ClassId>,
    /// The classes that derive from an exception class (CLS-03).
    pub(super) exceptions: BTreeMap<ClassId, ExceptionClass>,
}

impl Program {
    pub(crate) fn new(
        module: &str,
        definitions: &[Definition<'_>],
        module_variables: BTreeSet<String>,
        classes: Vec<ClassInfo>,
        class_ids: BTreeMap<String, ClassId>,
        exceptions: BTreeMap<ClassId, ExceptionClass>,
    ) -> Self {
        // Methods are reached through their instances, so only module functions have names.
        let function_ids = definitions
            .iter()
            .enumerate()
            .filter(|(_, definition)| definition.class.is_none())
            .map(|(index, definition)| (definition.name.clone(), FunctionId(id(index))))
            .collect();
        Self {
            module: module.to_owned(),
            symbols: Vec::new(),
            functions: vec![None; definitions.len()],
            function_ids,
            signatures: definitions
                .iter()
                .map(|definition| definition.signature.clone())
                .collect(),
            module_variables,
            classes,
            class_ids,
            exceptions,
        }
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

    pub(crate) fn finish(self, entry: FunctionId) -> TypedProgram {
        let classes = self
            .classes
            .iter()
            .enumerate()
            .map(|(index, class)| TypedClass {
                id: ClassId(id(index)),
                name: class.name.clone(),
                module: self.module.clone(),
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
            entry,
        }
    }
}

pub(super) fn id(index: usize) -> u32 {
    u32::try_from(index).unwrap_or(u32::MAX)
}
