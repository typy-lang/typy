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
    fn visit(&self, each: &mut impl FnMut(&Self)) {
        each(self);
        match self {
            Self::List(item)
            | Self::TupleOf(item)
            | Self::Set(item)
            | Self::Optional(item)
            | Self::Iterator(item)
            | Self::Iterable(item) => item.visit(each),
            Self::Dict(key, value) | Self::DictView(_, key, value) => {
                key.visit(each);
                value.visit(each);
            }
            Self::Tuple(items) | Self::Union(items) => {
                for item in items {
                    item.visit(each);
                }
            }
            _ => {}
        }
    }

    /// Returns the type with each parameter of `owner` replaced by its argument.
    #[must_use]
    pub fn substituted(&self, owner: Span, arguments: &[Type]) -> Self {
        let inner = |item: &Self| Box::new(item.substituted(owner, arguments));
        match self {
            // A parameter of another definition stays as it is.
            Self::Parameter(parameter) => arguments
                .get(parameter.index)
                .filter(|_| parameter.owner == owner)
                .cloned()
                .unwrap_or_else(|| self.clone()),
            Self::List(item) => Self::List(inner(item)),
            Self::TupleOf(item) => Self::TupleOf(inner(item)),
            Self::Set(item) => Self::Set(inner(item)),
            Self::Iterator(item) => Self::Iterator(inner(item)),
            Self::Iterable(item) => Self::Iterable(inner(item)),
            Self::Optional(item) => Self::optional(item.substituted(owner, arguments)),
            Self::Dict(key, value) => Self::Dict(inner(key), inner(value)),
            Self::DictView(view, key, value) => Self::DictView(*view, inner(key), inner(value)),
            Self::Tuple(items) => Self::Tuple(
                items
                    .iter()
                    .map(|item| item.substituted(owner, arguments))
                    .collect(),
            ),
            Self::Union(items) => Self::union(
                items
                    .iter()
                    .map(|item| item.substituted(owner, arguments))
                    .collect(),
            ),
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
        (Type::Iterable(item), given) => {
            if let Some(given) = given.iterable_item() {
                infer(item, &given, owner, solved);
            }
        }
        // `None` solves nothing, and a value of the base solves the base.
        (Type::Optional(_), Type::NoneType) => {}
        (Type::Optional(item), given) => infer(item, given, owner, solved),
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

#[cfg(test)]
mod tests {
    use super::{TypeParameter, infer};
    use crate::types::Type;
    use typy_diagnostics::{FileId, Span, TextOffset};

    fn parameter(index: usize, bound: Option<Type>) -> Type {
        Type::parameter(TypeParameter {
            owner: Span::empty(FileId(0), TextOffset(4)),
            index,
            name: format!("T{index}"),
            bound,
            constraints: Vec::new(),
        })
    }

    fn owner() -> Span {
        Span::empty(FileId(0), TextOffset(4))
    }

    #[test]
    fn arguments_solve_the_parameters_they_meet() {
        let (first, second) = (parameter(0, None), parameter(1, None));
        let mut solved = vec![None, None];
        let pairs = Type::Dict(
            Box::new(first.clone()),
            Box::new(Type::List(Box::new(second.clone()))),
        );
        let given = Type::Dict(
            Box::new(Type::Str),
            Box::new(Type::List(Box::new(Type::Int))),
        );
        infer(&pairs, &given, owner(), &mut solved);
        assert_eq!(solved, vec![Some(Type::Str), Some(Type::Int)]);
        let mut solved = vec![None, None];
        infer(&first, &Type::Int, owner(), &mut solved);
        infer(&first, &Type::Str, owner(), &mut solved);
        infer(&first, &Type::Int, owner(), &mut solved);
        assert_eq!(solved[0], Some(Type::union(vec![Type::Int, Type::Str])));
        let optional = Type::optional(second.clone());
        infer(&optional, &Type::NoneType, owner(), &mut solved);
        assert_eq!(solved[1], None);
        infer(&optional, &Type::Bytes, owner(), &mut solved);
        assert_eq!(solved[1], Some(Type::Bytes));
    }

    #[test]
    fn a_parameter_is_held_as_itself_or_where_its_bound_is() {
        let free = parameter(0, None);
        let bounded = parameter(1, Some(Type::Str));
        assert!(free.is_assignable_to(&free));
        assert!(!free.is_assignable_to(&Type::Str));
        assert!(bounded.is_assignable_to(&Type::Str));
        assert!(bounded.is_upcast_to(&Type::Str));
        assert!(!bounded.is_upcast_to(&bounded));
        assert!(!Type::Str.is_assignable_to(&bounded));
        assert_eq!(bounded.with_article(), "a value of type T1");
        // A parameter of another definition is neither solved nor replaced here.
        let other = Type::parameter(TypeParameter {
            owner: Span::empty(FileId(0), TextOffset(9)),
            index: 0,
            name: "U".to_owned(),
            bound: None,
            constraints: Vec::new(),
        });
        let mut solved = vec![None];
        infer(&other, &Type::Int, owner(), &mut solved);
        assert_eq!(solved, vec![None]);
        assert_eq!(other.substituted(owner(), &[Type::Int]), other);
        // A wider second value widens the solution.
        let mut solved = vec![Some(Type::Bool)];
        infer(
            &free,
            &Type::union(vec![Type::Bool, Type::Str]),
            owner(),
            &mut solved,
        );
        assert_eq!(solved, vec![Some(Type::union(vec![Type::Bool, Type::Str]))]);
    }

    #[test]
    fn a_substitution_replaces_each_parameter_in_place() {
        let first = parameter(0, None);
        let shape = Type::Tuple(vec![
            Type::List(Box::new(first.clone())),
            Type::optional(first.clone()),
            Type::Str,
        ]);
        let made = shape.substituted(owner(), &[Type::Int]);
        assert_eq!(
            made,
            Type::Tuple(vec![
                Type::List(Box::new(Type::Int)),
                Type::optional(Type::Int),
                Type::Str
            ])
        );
        assert!(shape.has_parameter());
        assert!(!made.has_parameter());
        assert_eq!(first.substituted(owner(), &[]), first);
    }
}
