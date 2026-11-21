//! The copies of generic functions: one for each set of type arguments the program reaches,
//! each checked as an ordinary function (GEN-03, ADR-0023).

use std::collections::BTreeMap;

use typy_diagnostics::{FileId, Span, TextOffset};

use super::Program;
use crate::generic::TypeParameter;
use crate::typed::{FunctionId, FunctionSignature, SignatureParameter};
use crate::types::Type;

/// The most copies made one inside another; a copy that needs a larger copy of itself would
/// never end (ADR-0023).
pub(crate) const MOST_NESTED_COPIES: usize = 64;

/// A generic function of the program, with the module that defines it, and whether it is
/// nested in a function, whose body checks its copies (FUN-18).
#[derive(Clone, Debug)]
pub(crate) struct Generic {
    pub(crate) parameters: Vec<TypeParameter>,
    pub(crate) module: String,
    pub(crate) nested: bool,
}

/// A copy of a generic function for its type arguments.
#[derive(Clone, Debug)]
pub(crate) struct GenericCopy {
    pub(crate) generic: FunctionId,
    pub(crate) arguments: Vec<Type>,
    pub(crate) copy: FunctionId,
    /// How many copies it is made inside: 1 for one a call outside any copy needs.
    pub(crate) depth: usize,
    /// For a copy of a method of a generic class, the class copy it belongs to (ADR-0028).
    pub(crate) class: Option<crate::typed::ClassId>,
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
                        nested: false,
                    },
                );
            }
        }
    }

    /// Returns the copy of `generic` for `arguments`, making it when it is new, as a method of
    /// the copy of a generic class `class` when it is one; a new copy is checked when the
    /// module's other bodies are.
    pub(crate) fn request_copy(
        &mut self,
        generic: FunctionId,
        arguments: Vec<Type>,
        class: Option<crate::typed::ClassId>,
    ) -> Requested {
        let known = self
            .generics
            .copies
            .iter()
            .any(|copy| copy.generic == generic && copy.arguments == arguments);
        if !known && self.generics.depth >= MOST_NESTED_COPIES {
            return Requested::TooDeep;
        }
        Requested::Copy(self.made_copy(generic, arguments, class))
    }

    /// Returns the copy of `generic` for `arguments`, making it when it is new; a copy of a
    /// method of a generic class names the class copy it belongs to.
    pub(crate) fn made_copy(
        &mut self,
        generic: FunctionId,
        arguments: Vec<Type>,
        class: Option<crate::typed::ClassId>,
    ) -> FunctionId {
        if let Some(known) = self
            .generics
            .copies
            .iter()
            .find(|copy| copy.generic == generic && copy.arguments == arguments)
        {
            return known.copy;
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
            depth: self.generics.depth + 1,
            class,
        });
        copy
    }

    /// Returns the signature of `generic` with each type parameter replaced by its argument.
    pub(crate) fn instantiated(
        &self,
        generic: FunctionId,
        arguments: &[Type],
    ) -> FunctionSignature {
        let signature = &self.signatures[generic.0 as usize];
        let parameters = self
            .generics
            .functions
            .get(&generic)
            .map(|known| known.parameters.clone())
            .unwrap_or_default();
        FunctionSignature {
            parameters: signature
                .parameters
                .iter()
                .map(|parameter| SignatureParameter {
                    ty: self.substitute_each(&parameter.ty, &parameters, arguments),
                    ..parameter.clone()
                })
                .collect(),
            returns: self.substitute_each(&signature.returns, &parameters, arguments),
        }
    }

    /// Returns `ty` with each of `parameters` replaced by the argument at its position. A
    /// method of a generic class with type parameters of its own has parameters of two
    /// definitions, the class's first, each replaced in turn (GEN-09).
    pub(crate) fn substitute_each(
        &self,
        ty: &Type,
        parameters: &[TypeParameter],
        arguments: &[Type],
    ) -> Type {
        let mut owners: Vec<Span> = Vec::new();
        for parameter in parameters {
            if !owners.contains(&parameter.owner) {
                owners.push(parameter.owner);
            }
        }
        // A function with no type parameter is the same for any arguments.
        if owners.is_empty() {
            return self.substitute(ty, Span::empty(FileId(0), TextOffset(0)), arguments);
        }
        owners.iter().fold(ty.clone(), |ty, owner| {
            let mut given: Vec<Type> = Vec::new();
            for (parameter, argument) in parameters.iter().zip(arguments) {
                if parameter.owner == *owner {
                    if given.len() <= parameter.index {
                        given.resize(parameter.index + 1, Type::Error);
                    }
                    given[parameter.index] = argument.clone();
                }
            }
            self.substitute(&ty, *owner, &given)
        })
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
