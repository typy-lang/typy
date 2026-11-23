//! `typing.Self` as a type (TYP-30, ADR-0113): in a method's parameters and result, the
//! class the method is called through. A value of it is an instance of the method's class,
//! held as one, so an expression's type is never `Self`; a declared type is, and a call
//! gives it the type of the instance or the class it is made through.

use crate::types::Type;

/// The name an annotation scope gives `Self` among its type parameters; a name with a space
/// is no Python name, so it never meets one the program writes.
pub(crate) const SELF_PARAMETER: &str = " Self";

impl Type {
    /// Returns `Self` of the class whose instances are of type `instance`.
    #[must_use]
    pub fn self_of(instance: Self) -> Self {
        Self::SelfType(Box::new(instance))
    }

    /// Returns true when the type is, or holds, `Self`.
    #[must_use]
    pub fn has_self(&self) -> bool {
        let mut found = false;
        self.visit(&mut |ty| found |= matches!(ty, Self::SelfType(_)));
        found
    }

    /// Returns the type with `Self` replaced by `receiver`, the type of the instance a call
    /// is made through, or by the class's own instances when `receiver` is `None`.
    #[must_use]
    pub fn with_self(&self, receiver: Option<&Self>) -> Self {
        if !self.has_self() {
            return self.clone();
        }
        let inner = |item: &Self| Box::new(item.with_self(receiver));
        match self {
            Self::SelfType(own) => receiver.cloned().unwrap_or_else(|| own.as_ref().clone()),
            Self::Optional(item) => Self::optional(*inner(item)),
            Self::Union(items) => Self::union(items.iter().map(|item| *inner(item)).collect()),
            Self::List(item) => Self::List(inner(item)),
            Self::TupleOf(item) => Self::TupleOf(inner(item)),
            Self::Set(item) => Self::Set(inner(item)),
            Self::Iterator(item) => Self::Iterator(inner(item)),
            Self::Iterable(item) => Self::Iterable(inner(item)),
            Self::Dict(key, value) => Self::Dict(inner(key), inner(value)),
            Self::Tuple(items) => Self::Tuple(items.iter().map(|item| *inner(item)).collect()),
            Self::Callable {
                parameters,
                returns,
            } => Self::Callable {
                parameters: parameters.iter().map(|item| *inner(item)).collect(),
                returns: inner(returns),
            },
            other => other.clone(),
        }
    }

    /// Returns true when the type holds `Self` other than as itself or the value of an
    /// optional, the forms a call reads as the type of what it is made through.
    #[must_use]
    pub fn has_nested_self(&self) -> bool {
        match self {
            Self::SelfType(_) => false,
            Self::Optional(inner) => inner.has_nested_self(),
            other => other.has_self(),
        }
    }
}

#[cfg(test)]
mod tests;
