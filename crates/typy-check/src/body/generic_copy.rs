//! The copies of generic functions: one for each set of type arguments the program reaches,
//! each checked as an ordinary function (GEN-03, ADR-0023).

use std::collections::BTreeMap;

use super::Program;
use crate::generic::TypeParameter;
use crate::typed::{FunctionId, FunctionSignature, SignatureParameter};
use crate::types::Type;

/// The most copies made one inside another; a copy that needs a larger copy of itself would
/// never end (ADR-0023).
pub(crate) const MOST_NESTED_COPIES: usize = 64;

/// A generic function of the program, with the module that defines it.
#[derive(Clone, Debug)]
pub(crate) struct Generic {
    pub(crate) parameters: Vec<TypeParameter>,
    pub(crate) module: String,
}

/// A copy of a generic function for its type arguments.
#[derive(Clone, Debug)]
pub(crate) struct GenericCopy {
    pub(crate) generic: FunctionId,
    pub(crate) arguments: Vec<Type>,
    pub(crate) copy: FunctionId,
    /// How many copies it is made inside: 1 for one a call outside any copy needs.
    pub(crate) depth: usize,
}

/// What asking for a copy gives.
pub(crate) enum Requested {
    /// The copy, made now or before.
    Copy(FunctionId),
    /// The chain of copies would pass `MOST_NESTED_COPIES`.
    TooDeep,
}

/// The generic functions and their copies.
#[derive(Clone, Debug, Default)]
pub(crate) struct Generics {
    pub(crate) functions: BTreeMap<FunctionId, Generic>,
    pub(crate) copies: Vec<GenericCopy>,
    /// The copies made and not yet checked, by position in `copies`.
    pub(crate) pending: Vec<usize>,
    /// The depth of the copy being checked, 0 outside any copy.
    pub(crate) depth: usize,
}

impl Program {
    /// Notes the generic functions of the module about to be checked, from their first ID.
    pub(crate) fn note_generics(
        &mut self,
        first: usize,
        definitions: &[crate::check::Definition<'_>],
    ) {
        for (index, definition) in definitions.iter().enumerate() {
            if !definition.type_parameters.is_empty() {
                let id = FunctionId(u32::try_from(first + index).unwrap_or(u32::MAX));
                self.generics.functions.insert(
                    id,
                    Generic {
                        parameters: definition.type_parameters.clone(),
                        module: self.module.clone(),
                    },
                );
            }
        }
    }

    /// Returns the copy of `generic` for `arguments`, making it when it is new; a new copy is
    /// checked when the module's other bodies are.
    pub(crate) fn request_copy(&mut self, generic: FunctionId, arguments: Vec<Type>) -> Requested {
        if let Some(known) = self
            .generics
            .copies
            .iter()
            .find(|copy| copy.generic == generic && copy.arguments == arguments)
        {
            return Requested::Copy(known.copy);
        }
        let depth = self.generics.depth + 1;
        if depth > MOST_NESTED_COPIES {
            return Requested::TooDeep;
        }
        let copy = self.next_function();
        let signature = self.instantiated(generic, &arguments);
        self.functions.push(None);
        self.signatures.push(signature);
        self.generics.pending.push(self.generics.copies.len());
        self.generics.copies.push(GenericCopy {
            generic,
            arguments,
            copy,
            depth,
        });
        Requested::Copy(copy)
    }

    /// Returns the signature of `generic` with each type parameter replaced by its argument.
    pub(crate) fn instantiated(
        &self,
        generic: FunctionId,
        arguments: &[Type],
    ) -> FunctionSignature {
        let signature = &self.signatures[generic.0 as usize];
        let Some(owner) = self
            .generics
            .functions
            .get(&generic)
            .and_then(|known| known.parameters.first())
            .map(|parameter| parameter.owner)
        else {
            return signature.clone();
        };
        FunctionSignature {
            parameters: signature
                .parameters
                .iter()
                .map(|parameter| SignatureParameter {
                    ty: parameter.ty.substituted(owner, arguments),
                    ..parameter.clone()
                })
                .collect(),
            returns: signature.returns.substituted(owner, arguments),
        }
    }

    /// Returns the next copy to check, if any, and makes its depth the current one.
    pub(crate) fn next_pending_copy(&mut self) -> Option<GenericCopy> {
        let index = self.generics.pending.pop()?;
        let copy = self.generics.copies[index].clone();
        self.generics.depth = copy.depth;
        Some(copy)
    }

    /// Ends checking copies: calls outside any copy are at depth 0 again.
    pub(crate) fn end_copies(&mut self) {
        self.generics.depth = 0;
    }

    /// Returns the type parameters of a generic function, or `None` for any other.
    pub(crate) fn generic(&self, function: FunctionId) -> Option<&Generic> {
        self.generics.functions.get(&function)
    }
}
