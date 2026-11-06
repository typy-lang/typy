//! The static types of release 0.1.

use std::fmt;

/// The static type of a value (TYP-11).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Type {
    /// `int`, without a size limit (DAT-01).
    Int,
    /// `float`, an IEEE 754 double (DAT-02).
    Float,
    /// `bool` (DAT-03).
    Bool,
    /// `str` (DAT-05).
    Str,
    /// The type of `None` (DAT-04).
    NoneType,
    /// The type of an expression that already has a diagnostic; every check accepts it, so one cause gives one error (DIA-04).
    Error,
}

impl Type {
    /// Returns true when a value of this type may be stored where `target` is declared (TYP-03, TYP-09).
    #[must_use]
    pub fn is_assignable_to(self, target: Self) -> bool {
        self == target || self == Self::Error || target == Self::Error
    }

    /// Returns true for `int` and `float`.
    #[must_use]
    pub fn is_numeric(self) -> bool {
        matches!(self, Self::Int | Self::Float)
    }

    /// Returns true when a value of this type may be used in a truth test (EXP-10).
    #[must_use]
    pub fn is_truth_testable(self) -> bool {
        true
    }

    /// Returns the type's name with its article, for messages: `an int`, `a str`, `None`.
    #[must_use]
    pub fn with_article(self) -> String {
        match self {
            Self::Int => "an int".to_owned(),
            Self::NoneType => "None".to_owned(),
            other => format!("a {other}"),
        }
    }

    /// Returns true when `print`, `str` and `repr` accept a value of this type.
    #[must_use]
    pub fn is_printable(self) -> bool {
        true
    }
}

impl fmt::Display for Type {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Int => "int",
            Self::Float => "float",
            Self::Bool => "bool",
            Self::Str => "str",
            Self::NoneType => "None",
            Self::Error => "<error>",
        })
    }
}

#[cfg(test)]
mod tests {
    use super::Type;

    #[test]
    fn an_int_is_not_assignable_to_a_float() {
        assert!(!Type::Int.is_assignable_to(Type::Float));
        assert!(!Type::Float.is_assignable_to(Type::Int));
        assert!(!Type::Bool.is_assignable_to(Type::Int));
    }

    #[test]
    fn equal_types_and_error_are_assignable() {
        for ty in [
            Type::Int,
            Type::Float,
            Type::Bool,
            Type::Str,
            Type::NoneType,
        ] {
            assert!(ty.is_assignable_to(ty));
            assert!(Type::Error.is_assignable_to(ty));
            assert!(ty.is_assignable_to(Type::Error));
            assert!(ty.is_truth_testable());
            assert!(ty.is_printable());
        }
    }

    #[test]
    fn names_take_the_right_article() {
        assert_eq!(Type::Int.with_article(), "an int");
        assert_eq!(Type::Str.with_article(), "a str");
        assert_eq!(Type::NoneType.with_article(), "None");
    }

    #[test]
    fn only_int_and_float_are_numeric() {
        assert!(Type::Int.is_numeric() && Type::Float.is_numeric());
        assert!(!Type::Bool.is_numeric() && !Type::Str.is_numeric());
    }

    #[test]
    fn types_are_written_with_their_python_names() {
        let names: Vec<String> = [
            Type::Int,
            Type::Float,
            Type::Bool,
            Type::Str,
            Type::NoneType,
        ]
        .iter()
        .map(ToString::to_string)
        .collect();
        assert_eq!(names, ["int", "float", "bool", "str", "None"]);
    }
}
