//! The program being built while bodies are checked: symbols, finished functions and what every body may refer to.

use std::collections::{BTreeMap, BTreeSet};

use typy_diagnostics::Span;

use crate::check::{ClassInfo, Definition};
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
}

impl Program {
    pub(crate) fn new(
        module: &str,
        definitions: &[Definition<'_>],
        module_variables: BTreeSet<String>,
        classes: Vec<ClassInfo>,
        class_ids: BTreeMap<String, ClassId>,
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
        }
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
        let module = self.module;
        let classes = self
            .classes
            .into_iter()
            .enumerate()
            .map(|(index, class)| TypedClass {
                id: ClassId(id(index)),
                name: class.name,
                module: module.clone(),
                attributes: class
                    .attributes
                    .into_iter()
                    .map(|attribute| Attribute {
                        name: attribute.name,
                        ty: attribute.ty.unwrap_or(crate::types::Type::Error),
                    })
                    .collect(),
                methods: class.methods,
                init: class.init,
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
