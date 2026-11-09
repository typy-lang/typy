//! The static types: the scalars of 0.1, and from 0.2 the types built from other types.

use std::fmt;

/// The static type of a value (TYP-11). Types built from others, such as a tuple, hold their
/// parts, and two types are equal when their structure is.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
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
    /// `tuple[A, B, ...]`: a fixed number of items, each with its own type (DAT-08).
    Tuple(Vec<Type>),
    /// `list[T]`: a mutable sequence shared by every name bound to it (DAT-07).
    List(Box<Type>),
    /// `tuple[T, ...]`: an immutable sequence of any number of items of one type (DAT-14).
    TupleOf(Box<Type>),
    /// The type of an expression that already has a diagnostic; every check accepts it, so one cause gives one error (DIA-04).
    Error,
}

impl Type {
    /// Returns true when a value of this type may be stored where `target` is declared, item by
    /// item for a tuple (TYP-03, TYP-09).
    #[must_use]
    pub fn is_assignable_to(&self, target: &Self) -> bool {
        match (self, target) {
            (Self::Error, _) | (_, Self::Error) => true,
            (Self::List(item), Self::List(target))
            | (Self::TupleOf(item), Self::TupleOf(target)) => item.is_assignable_to(target),
            (Self::Tuple(items), Self::Tuple(targets)) => {
                items.len() == targets.len()
                    && items
                        .iter()
                        .zip(targets)
                        .all(|(item, target)| item.is_assignable_to(target))
            }
            _ => self == target,
        }
    }

    /// Returns true when the type is `Error`, or holds it, so a check of it would repeat an error.
    #[must_use]
    pub fn has_error(&self) -> bool {
        match self {
            Self::Error => true,
            Self::Tuple(items) => items.iter().any(Self::has_error),
            Self::List(item) | Self::TupleOf(item) => item.has_error(),
            _ => false,
        }
    }

    /// Returns true when `==` and `!=` accept a value of this type against one of `other`: the
    /// same type, two numbers, or tuples whose items accept each other (EXP-06).
    #[must_use]
    pub fn can_test_equality(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Error, _) | (_, Self::Error) => true,
            // The runtime compares items by the list's own item kind, so two lists compare when
            // their items have one type.
            (Self::List(left), Self::List(right)) | (Self::TupleOf(left), Self::TupleOf(right)) => {
                left == right
            }
            (Self::Tuple(left), Self::Tuple(right)) => {
                left.len() == right.len()
                    && left
                        .iter()
                        .zip(right)
                        .all(|(left, right)| left.can_test_equality(right))
            }
            _ => self == other || (self.is_numeric() && other.is_numeric()),
        }
    }

    /// Returns true when `<`, `<=`, `>` and `>=` accept a value of this type against one of
    /// `other`: the same type other than `None`, two numbers, or tuples whose items order
    /// against each other (EXP-06). CPython orders tuples by the first pair of items that are
    /// not equal, so a pair of `None` items, which are always equal, is never ordered and is
    /// accepted inside a tuple.
    #[must_use]
    pub fn can_order(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Error, _) | (_, Self::Error) => true,
            (Self::TupleOf(left), Self::TupleOf(right)) => left == right && left.can_order(right),
            (Self::Tuple(left), Self::Tuple(right)) => {
                left.len() == right.len()
                    && left.iter().zip(right).all(|(left, right)| {
                        (*left == Self::NoneType && *right == Self::NoneType)
                            || left.can_order(right)
                    })
            }
            (Self::NoneType, _) | (_, Self::NoneType) => false,
            _ => self == other || (self.is_numeric() && other.is_numeric()),
        }
    }

    /// Returns the item type of a sequence stored as a runtime list object: a `list[T]` or a
    /// `tuple[T, ...]`.
    #[must_use]
    pub fn sequence_item(&self) -> Option<&Self> {
        match self {
            Self::List(item) | Self::TupleOf(item) => Some(item),
            _ => None,
        }
    }

    /// Returns `tuple[T, ...]` for a fixed tuple whose items are all of type `item`, which it
    /// converts to where that type is expected (DAT-14).
    #[must_use]
    pub fn fits_tuple_of(&self, item: &Self) -> bool {
        matches!(self, Self::Tuple(items) if items.iter().all(|each| each.is_assignable_to(item)))
    }

    /// Returns true for `int` and `float`.
    #[must_use]
    pub fn is_numeric(&self) -> bool {
        matches!(self, Self::Int | Self::Float)
    }

    /// Returns true when a value of this type may be used in a truth test (EXP-10).
    #[must_use]
    pub fn is_truth_testable(&self) -> bool {
        true
    }

    /// Returns the type's name with its article, for messages: `an int`, `a str`, `None`.
    #[must_use]
    pub fn with_article(&self) -> String {
        match self {
            Self::Int => "an int".to_owned(),
            Self::NoneType => "None".to_owned(),
            other => format!("a {other}"),
        }
    }

    /// Returns true when `print`, `str` and `repr` accept a value of this type.
    #[must_use]
    pub fn is_printable(&self) -> bool {
        true
    }
}

impl fmt::Display for Type {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Int => formatter.write_str("int"),
            Self::Float => formatter.write_str("float"),
            Self::Bool => formatter.write_str("bool"),
            Self::Str => formatter.write_str("str"),
            Self::NoneType => formatter.write_str("None"),
            Self::Tuple(items) if items.is_empty() => formatter.write_str("tuple[()]"),
            Self::Tuple(items) => {
                formatter.write_str("tuple[")?;
                for (index, item) in items.iter().enumerate() {
                    if index > 0 {
                        formatter.write_str(", ")?;
                    }
                    write!(formatter, "{item}")?;
                }
                formatter.write_str("]")
            }
            Self::List(item) => write!(formatter, "list[{item}]"),
            Self::TupleOf(item) => write!(formatter, "tuple[{item}, ...]"),
            Self::Error => formatter.write_str("<error>"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Type;

    #[test]
    fn an_int_is_not_assignable_to_a_float() {
        assert!(!Type::Int.is_assignable_to(&Type::Float));
        assert!(!Type::Float.is_assignable_to(&Type::Int));
        assert!(!Type::Bool.is_assignable_to(&Type::Int));
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
            assert!(ty.is_assignable_to(&ty));
            assert!(Type::Error.is_assignable_to(&ty));
            assert!(ty.is_assignable_to(&Type::Error));
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
        let numeric: Vec<bool> = [
            Type::Int,
            Type::Float,
            Type::Bool,
            Type::Str,
            Type::NoneType,
        ]
        .iter()
        .map(Type::is_numeric)
        .collect::<Vec<_>>();
        assert_eq!(numeric, [true, true, false, false, false]);
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
