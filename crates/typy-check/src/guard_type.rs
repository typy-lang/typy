//! `typing.TypeGuard[T]` and `typing.TypeIs[T]` as a function's result (TYP-31, ADR-0114):
//! the function returns a `bool`, and a test of a call of it narrows the first argument, a
//! variable, to `T` where the call is true, and for `TypeIs` to the rest where it is false.
//! A value of either is a `bool`, so an expression's type is never one; a declared result
//! is.

use std::fmt;

use crate::types::Type;

/// Which of the two forms a guard is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum GuardKind {
    /// `TypeGuard[T]`: the argument is a `T` where the call is true.
    TypeGuard,
    /// `TypeIs[T]`: the argument is a `T` exactly where the call is true.
    TypeIs,
}

/// A declared guard: its form and the type it narrows to.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct GuardType {
    /// The form.
    pub kind: GuardKind,
    /// The type the argument narrows to.
    pub target: Type,
}

impl GuardKind {
    /// Returns the name of the form in `typing`.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::TypeGuard => "TypeGuard",
            Self::TypeIs => "TypeIs",
        }
    }
}

/// Returns whether a test of a value's class tells whether it is of `target`: each member of
/// it is a number, a `str`, `bytes`, `None`, a range, a slice, or an instance of a class,
/// which no type arguments tell apart further (TYP-31).
pub(crate) fn tested_by_class(target: &Type) -> bool {
    let members: Vec<Type> = match target {
        Type::Optional(inner) => inner.members().to_vec(),
        other => other.members().to_vec(),
    };
    members.iter().all(|member| {
        matches!(
            member,
            Type::Int
                | Type::Float
                | Type::Bool
                | Type::Str
                | Type::Bytes
                | Type::Bytearray
                | Type::NoneType
                | Type::Range
                | Type::Slice
                | Type::Exception(_)
        ) || matches!(member, Type::Instance(class) if class.generic.is_none() && !class.protocol)
    })
}

impl fmt::Display for GuardType {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}[{}]", self.kind.name(), self.target)
    }
}

impl Type {
    /// Returns the guard result of `kind` that narrows to `target`.
    #[must_use]
    pub fn guard(kind: GuardKind, target: Self) -> Self {
        Self::Guard(Box::new(GuardType { kind, target }))
    }

    /// Returns the type with a guard result as the `bool` a value of it is.
    #[must_use]
    pub fn without_guard(&self) -> Self {
        match self {
            Self::Guard(_) => Self::Bool,
            other => other.clone(),
        }
    }

    /// Returns the type with each declared form only the checker reads as the type of a
    /// value of it: a literal type as its base type, `Self` as the method's class, and a
    /// guard as `bool` (TYP-28, TYP-30, TYP-31).
    #[must_use]
    pub fn without_declared_forms(&self) -> Self {
        self.without_literal().with_self(None).without_guard()
    }
}

#[cfg(test)]
mod tests;
