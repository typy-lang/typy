//! The program being built while bodies are checked: symbols, finished functions and what every body may refer to.

use std::collections::{BTreeMap, BTreeSet};

use typy_diagnostics::Span;

use crate::check::Definition;
use crate::typed::{
    FunctionId, FunctionSignature, Symbol, SymbolId, SymbolKind, TypedFunction, TypedProgram,
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
}

impl Program {
    pub(crate) fn new(
        module: &str,
        definitions: &[Definition<'_>],
        module_variables: BTreeSet<String>,
    ) -> Self {
        let function_ids = definitions
            .iter()
            .enumerate()
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
        TypedProgram {
            symbols: self.symbols,
            functions: self.functions.into_iter().flatten().collect(),
            entry,
        }
    }
}

pub(super) fn id(index: usize) -> u32 {
    u32::try_from(index).unwrap_or(u32::MAX)
}
