//! Type parameters of generic functions, as PEP 695 writes them (GEN-01, GEN-02, ADR-0023):
//! the opaque type a generic body is checked with, the substitution of type arguments, and
//! their inference from a call's arguments.

use std::sync::Arc;

use typy_diagnostics::Span;

use crate::types::Type;

/// A type parameter of a generic function: `T`, `T: Bound` or `T: (A, B)`.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct TypeParameter {
    /// The definition the parameter belongs to, which tells it from another `T`.
    pub owner: Span,
    /// Its position in the definition's list.
    pub index: usize,
    /// Its name.
    pub name: String,
    /// The type every argument must be, when it has a bound (GEN-02).
    pub bound: Option<Type>,
    /// The types an argument is solved to, when it has constraints (GEN-02).
    pub constraints: Vec<Type>,
}

impl Type {
    /// Returns the opaque type of a type parameter, which offers what its bound offers.
    #[must_use]
    pub fn parameter(parameter: TypeParameter) -> Self {
        Self::Parameter(Arc::new(parameter))
    }

    /// Returns true when the type is, or holds, a type parameter.
    #[must_use]
    pub fn has_parameter(&self) -> bool {
        let mut found = false;
        self.visit(&mut |ty| found |= matches!(ty, Self::Parameter(_)));
        found
    }

    /// Calls `each` on the type and on every type inside it.
    pub(crate) fn visit(&self, each: &mut impl FnMut(&Self)) {
        each(self);
        match self {
            Self::List(item)
            | Self::TupleOf(item)
            | Self::Set(item)
            | Self::Optional(item)
            | Self::Iterator(item)
            | Self::Iterable(item)
            | Self::SelfType(item) => item.visit(each),
            Self::Dict(key, value) | Self::DictView(_, key, value) => {
                key.visit(each);
                value.visit(each);
            }
            Self::Tuple(items) | Self::Union(items) => {
                for item in items {
                    item.visit(each);
                }
            }
            Self::Abstract(collection) => {
                for item in &collection.arguments {
                    item.visit(each);
                }
                collection.members.visit(each);
            }
            Self::Callable {
                parameters,
                returns,
            } => {
                for parameter in parameters {
                    parameter.visit(each);
                }
                returns.visit(each);
            }
            Self::Generator {
                yields,
                sends,
                returns,
            } => {
                yields.visit(each);
                sends.visit(each);
                returns.visit(each);
            }
            // A copy of a generic class holds its type arguments (ADR-0028).
            Self::Instance(class) => {
                for item in class.generic.iter().flat_map(|(_, items)| items) {
                    item.visit(each);
                }
            }
            _ => {}
        }
    }

    /// Returns the type with each parameter of `owner` replaced by its argument.
    #[must_use]
    pub fn substituted(&self, owner: Span, arguments: &[Type]) -> Self {
        self.substituted_with(owner, arguments, &mut |_| None)
    }

    /// Returns the type with each parameter of `owner` replaced by its argument, and each
    /// instance of a class replaced by what `classes` gives for it, if anything: a copy of a
    /// generic class becomes the copy for the replaced arguments (ADR-0028).
    pub(crate) fn substituted_with(
        &self,
        owner: Span,
        arguments: &[Type],
        classes: &mut dyn FnMut(&crate::class_type::InstanceClass) -> Option<Type>,
    ) -> Self {
        let mut inner = |item: &Self| Box::new(item.substituted_with(owner, arguments, classes));
        match self {
            // A parameter of another definition stays as it is.
            Self::Parameter(parameter) => arguments
                .get(parameter.index)
                .filter(|_| parameter.owner == owner)
                .cloned()
                .unwrap_or_else(|| self.clone()),
            Self::Instance(class) => classes(class).unwrap_or_else(|| self.clone()),
            Self::List(item) => Self::List(inner(item)),
            Self::TupleOf(item) => Self::TupleOf(inner(item)),
            Self::Set(item) => Self::Set(inner(item)),
            Self::Iterator(item) => Self::Iterator(inner(item)),
            Self::Iterable(item) => Self::Iterable(inner(item)),
            Self::Optional(item) => Self::optional(*inner(item)),
            Self::Dict(key, value) => {
                let key = inner(key);
                Self::Dict(key, inner(value))
            }
            Self::DictView(view, key, value) => {
                let key = inner(key);
                Self::DictView(*view, key, inner(value))
            }
            Self::Tuple(items) => Self::Tuple(items.iter().map(|item| *inner(item)).collect()),
            Self::Union(items) => Self::union(items.iter().map(|item| *inner(item)).collect()),
            Self::Abstract(collection) => {
                Self::Abstract(Box::new(crate::abstract_type::AbstractType {
                    collection: collection.collection,
                    arguments: collection
                        .arguments
                        .iter()
                        .map(|item| *inner(item))
                        .collect(),
                    members: *inner(&collection.members),
                }))
            }
            Self::Callable {
                parameters,
                returns,
            } => Self::Callable {
                parameters: parameters.iter().map(|item| *inner(item)).collect(),
                returns: inner(returns),
            },
            Self::Generator {
                yields,
                sends,
                returns,
            } => Self::Generator {
                yields: inner(yields),
                sends: inner(sends),
                returns: inner(returns),
            },
            other => other.clone(),
        }
    }
}

/// Solves the parameters of `owner` that `parameter`, a parameter's type, holds, from
/// `argument`, the type of the value given for it. Two values that solve one parameter to
/// different types solve it to their union, as type checkers do.
pub(crate) fn infer(parameter: &Type, argument: &Type, owner: Span, solved: &mut [Option<Type>]) {
    match (parameter, argument) {
        (Type::Parameter(each), _) => {
            // A parameter of another definition is not solved here.
            if let Some(slot) = solved.get_mut(each.index).filter(|_| each.owner == owner) {
                *slot = Some(match slot.take() {
                    None => argument.clone(),
                    Some(known) if argument.is_assignable_to(&known) => known,
                    Some(known) if known.is_assignable_to(argument) => argument.clone(),
                    Some(known) => Type::union(vec![known, argument.clone()]),
                });
            }
        }
        (Type::List(item), Type::List(given))
        | (Type::TupleOf(item), Type::TupleOf(given))
        | (Type::Set(item), Type::Set(given))
        | (Type::Iterator(item), Type::Iterator(given))
        | (Type::Optional(item), Type::Optional(given)) => infer(item, given, owner, solved),
        // Two copies of one generic class solve by their type arguments (ADR-0028).
        (Type::Instance(wanted), Type::Instance(given))
            if let (Some((root, items)), Some((given_root, given_items))) =
                (&wanted.generic, &given.generic)
                && root == given_root =>
        {
            for (item, given) in items.iter().zip(given_items) {
                infer(item, given, owner, solved);
            }
        }
        (Type::Dict(key, value), Type::Dict(given_key, given_value)) => {
            infer(key, given_key, owner, solved);
            infer(value, given_value, owner, solved);
        }
        (Type::Tuple(items), Type::Tuple(given)) if items.len() == given.len() => {
            for (item, given) in items.iter().zip(given) {
                infer(item, given, owner, solved);
            }
        }
        (Type::TupleOf(item), Type::Tuple(given)) => {
            for given in given {
                infer(item, given, owner, solved);
            }
        }
        // A function solves by its parameters and its result (ADR-0081).
        (
            Type::Callable {
                parameters,
                returns,
            },
            Type::Callable {
                parameters: given,
                returns: given_returns,
            },
        ) if parameters.len() == given.len() => {
            for (item, given) in parameters.iter().zip(given) {
                infer(item, given, owner, solved);
            }
            infer(returns, given_returns, owner, solved);
        }
        (
            Type::Generator {
                yields,
                sends,
                returns,
            },
            Type::Generator {
                yields: given_yields,
                sends: given_sends,
                returns: given_returns,
            },
        ) => {
            infer(yields, given_yields, owner, solved);
            infer(sends, given_sends, owner, solved);
            infer(returns, given_returns, owner, solved);
        }
        // A generator is an iterator of what it yields.
        (Type::Iterator(item), Type::Generator { yields, .. }) => {
            infer(item, yields, owner, solved);
        }
        (Type::Iterable(item), given) => {
            if let Some(given) = given.iterable_item() {
                infer(item, &given, owner, solved);
            }
        }
        // `None` solves nothing, and a value of the base solves the base.
        (Type::Optional(_), Type::NoneType) => {}
        (Type::Optional(item), given) => infer(item, given, owner, solved),
        // An abstract collection is solved by the items of the collection given, or by the
        // keys and values of a mapping (LIB-20).
        (Type::Abstract(collection), given) => infer_abstract(collection, given, owner, solved),
        // A union with one member that holds parameters is solved by what the other members
        // do not take.
        (Type::Union(members), given) => {
            let open: Vec<&Type> = members
                .iter()
                .filter(|member| member.has_parameter())
                .collect();
            if let [open] = open.as_slice()
                && !members
                    .iter()
                    .filter(|member| !member.has_parameter())
                    .any(|member| given.is_assignable_to(member))
            {
                infer(open, given, owner, solved);
            }
        }
        _ => {}
    }
}

/// Solves the parameters an abstract collection's arguments hold from the collection given:
/// its items, or a mapping's keys and values (LIB-20).
fn infer_abstract(
    collection: &crate::abstract_type::AbstractType,
    given: &Type,
    owner: Span,
    solved: &mut [Option<Type>],
) {
    match (collection.arguments.as_slice(), given) {
        ([key, value], Type::Dict(given_key, given_value)) => {
            infer(key, given_key, owner, solved);
            infer(value, given_value, owner, solved);
        }
        ([item], given) => {
            if let Some(given) = given.iterable_item() {
                infer(item, &given, owner, solved);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests;
