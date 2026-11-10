//! The exception classes a program can name, with CPython's bases (EXC-02, EXC-10, ADR-0013).

use std::fmt;

/// A built-in exception class.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[allow(
    missing_docs,
    reason = "each variant is the Python exception class of the same name"
)]
pub enum BuiltinException {
    BaseException,
    Exception,
    ArithmeticError,
    AssertionError,
    AttributeError,
    LookupError,
    IndexError,
    KeyError,
    OverflowError,
    RuntimeError,
    NotImplementedError,
    RecursionError,
    StopIteration,
    TypeError,
    ValueError,
    UnicodeError,
    UnicodeDecodeError,
    UnicodeEncodeError,
    ZeroDivisionError,
}

impl BuiltinException {
    /// Every built-in exception class.
    pub const ALL: [Self; 19] = [
        Self::BaseException,
        Self::Exception,
        Self::ArithmeticError,
        Self::AssertionError,
        Self::AttributeError,
        Self::LookupError,
        Self::IndexError,
        Self::KeyError,
        Self::OverflowError,
        Self::RuntimeError,
        Self::NotImplementedError,
        Self::RecursionError,
        Self::StopIteration,
        Self::TypeError,
        Self::ValueError,
        Self::UnicodeError,
        Self::UnicodeDecodeError,
        Self::UnicodeEncodeError,
        Self::ZeroDivisionError,
    ];

    /// Returns the class a built-in name names, such as `ValueError`.
    #[must_use]
    pub fn named(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|class| class.name() == name)
    }

    /// Returns the class's name, which is also the runtime's symbol suffix.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::BaseException => "BaseException",
            Self::Exception => "Exception",
            Self::ArithmeticError => "ArithmeticError",
            Self::AssertionError => "AssertionError",
            Self::AttributeError => "AttributeError",
            Self::LookupError => "LookupError",
            Self::IndexError => "IndexError",
            Self::KeyError => "KeyError",
            Self::OverflowError => "OverflowError",
            Self::RuntimeError => "RuntimeError",
            Self::NotImplementedError => "NotImplementedError",
            Self::RecursionError => "RecursionError",
            Self::StopIteration => "StopIteration",
            Self::TypeError => "TypeError",
            Self::ValueError => "ValueError",
            Self::UnicodeError => "UnicodeError",
            Self::UnicodeDecodeError => "UnicodeDecodeError",
            Self::UnicodeEncodeError => "UnicodeEncodeError",
            Self::ZeroDivisionError => "ZeroDivisionError",
        }
    }

    /// Returns the base class, or `None` for `BaseException`, whose base is `object`.
    #[must_use]
    pub const fn base(self) -> Option<Self> {
        Some(match self {
            Self::BaseException => return None,
            Self::Exception => Self::BaseException,
            Self::ArithmeticError
            | Self::AssertionError
            | Self::AttributeError
            | Self::LookupError
            | Self::RuntimeError
            | Self::StopIteration
            | Self::TypeError
            | Self::ValueError => Self::Exception,
            Self::IndexError | Self::KeyError => Self::LookupError,
            Self::OverflowError | Self::ZeroDivisionError => Self::ArithmeticError,
            Self::NotImplementedError | Self::RecursionError => Self::RuntimeError,
            Self::UnicodeError => Self::ValueError,
            Self::UnicodeDecodeError | Self::UnicodeEncodeError => Self::UnicodeError,
        })
    }

    /// Returns true when a program may call the class to build an exception: every class but
    /// the Unicode errors, whose five arguments TyPy does not check (EXC-12).
    #[must_use]
    pub const fn can_be_called(self) -> bool {
        !matches!(self, Self::UnicodeDecodeError | Self::UnicodeEncodeError)
    }
}

/// The class of an exception: a built-in one.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum ExceptionClass {
    /// A built-in exception class.
    Builtin(BuiltinException),
}

impl ExceptionClass {
    /// Returns the class's name.
    #[must_use]
    pub const fn name(&self) -> &str {
        match self {
            Self::Builtin(class) => class.name(),
        }
    }

    /// Returns true when the class is `base` or derives from it, so an exception of the class
    /// may be stored where `base` is declared and `except base` catches it (EXC-01).
    #[must_use]
    pub fn derives_from(&self, base: &Self) -> bool {
        let (Self::Builtin(class), Self::Builtin(base)) = (self, base);
        let mut current = Some(*class);
        while let Some(class) = current {
            if class == *base {
                return true;
            }
            current = class.base();
        }
        false
    }
}

impl fmt::Display for ExceptionClass {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.name())
    }
}

#[cfg(test)]
mod tests {
    use super::{BuiltinException, ExceptionClass};

    #[test]
    fn every_class_is_found_by_its_name_and_reaches_base_exception() {
        for class in BuiltinException::ALL {
            assert_eq!(BuiltinException::named(class.name()), Some(class));
            assert!(
                ExceptionClass::Builtin(class)
                    .derives_from(&ExceptionClass::Builtin(BuiltinException::BaseException)),
                "{class:?}"
            );
        }
        assert_eq!(BuiltinException::named("Warning"), None);
    }

    #[test]
    fn a_class_derives_from_its_bases_only() {
        let derives = |class, base| {
            ExceptionClass::Builtin(class).derives_from(&ExceptionClass::Builtin(base))
        };
        assert!(derives(
            BuiltinException::KeyError,
            BuiltinException::LookupError
        ));
        assert!(derives(
            BuiltinException::UnicodeDecodeError,
            BuiltinException::ValueError
        ));
        assert!(derives(
            BuiltinException::NotImplementedError,
            BuiltinException::RuntimeError
        ));
        assert!(!derives(
            BuiltinException::KeyError,
            BuiltinException::IndexError
        ));
        assert!(!derives(
            BuiltinException::Exception,
            BuiltinException::ValueError
        ));
    }

    #[test]
    fn only_the_unicode_errors_cannot_be_called() {
        let refused: Vec<_> = BuiltinException::ALL
            .into_iter()
            .filter(|class| !class.can_be_called())
            .collect();
        assert_eq!(
            refused,
            [
                BuiltinException::UnicodeDecodeError,
                BuiltinException::UnicodeEncodeError
            ]
        );
    }
}
