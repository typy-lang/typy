//! The exception classes a program can name, with CPython's bases (EXC-02, EXC-10, ADR-0013).

use std::fmt;
use std::sync::Arc;

use crate::typed::ClassId;

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

/// The class of an exception: a built-in one, or one of the program's (CLS-03).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum ExceptionClass {
    /// A built-in exception class.
    Builtin(BuiltinException),
    /// A class of the program derived from an exception class, shared so a type stays small.
    Class(Arc<ProgramException>),
}

/// A class of the program derived from an exception class (CLS-03).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ProgramException {
    /// The class.
    pub id: ClassId,
    /// Its name.
    pub name: String,
    /// Its bases that are classes of the program, nearest first, each with its name.
    pub ancestors: Vec<(ClassId, String)>,
    /// The built-in class its chain of bases reaches.
    pub builtin: BuiltinException,
}

impl ExceptionClass {
    /// Returns a class of the program.
    #[must_use]
    pub fn program(
        id: ClassId,
        name: String,
        ancestors: Vec<(ClassId, String)>,
        builtin: BuiltinException,
    ) -> Self {
        Self::Class(Arc::new(ProgramException {
            id,
            name,
            ancestors,
            builtin,
        }))
    }

    /// Returns the ID of a class of the program, or `None` for a built-in class.
    #[must_use]
    pub fn class_id(&self) -> Option<ClassId> {
        match self {
            Self::Builtin(_) => None,
            Self::Class(class) => Some(class.id),
        }
    }

    /// Returns the class's name.
    #[must_use]
    pub fn name(&self) -> &str {
        match self {
            Self::Builtin(class) => class.name(),
            Self::Class(class) => &class.name,
        }
    }

    /// Returns the built-in class at the root of the class's chain of bases.
    #[must_use]
    pub fn builtin(&self) -> BuiltinException {
        match self {
            Self::Builtin(class) => *class,
            Self::Class(class) => class.builtin,
        }
    }

    /// Returns the class's base, or `None` for `BaseException`.
    #[must_use]
    pub fn base(&self) -> Option<Self> {
        match self {
            Self::Builtin(class) => class.base().map(Self::Builtin),
            Self::Class(class) => Some(match class.ancestors.split_first() {
                Some(((id, name), rest)) => {
                    Self::program(*id, name.clone(), rest.to_vec(), class.builtin)
                }
                None => Self::Builtin(class.builtin),
            }),
        }
    }

    /// Returns true when the class is `base` or derives from it, so an exception of the class
    /// may be stored where `base` is declared and `except base` catches it (EXC-01).
    #[must_use]
    pub fn derives_from(&self, base: &Self) -> bool {
        match (self, base) {
            (Self::Class(class), Self::Class(base)) => {
                class.id == base.id
                    || class
                        .ancestors
                        .iter()
                        .any(|(ancestor, _)| *ancestor == base.id)
            }
            (Self::Builtin(_), Self::Class(_)) => false,
            (_, Self::Builtin(base)) => {
                let mut current = Some(self.builtin());
                while let Some(class) = current {
                    if class == *base {
                        return true;
                    }
                    current = class.base();
                }
                false
            }
        }
    }
}

/// Returns the nearest class every one of `classes` derives from, `BaseException` at the
/// farthest, as the type of the name an `except` clause with several classes binds.
#[must_use]
pub(crate) fn common_base(classes: &[ExceptionClass]) -> ExceptionClass {
    let mut candidate = classes.first().cloned();
    while let Some(base) = candidate {
        if classes.iter().all(|class| class.derives_from(&base)) {
            return base;
        }
        candidate = base.base();
    }
    ExceptionClass::Builtin(BuiltinException::BaseException)
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
    fn the_common_base_is_the_nearest_class_all_derive_from() {
        let common = |classes: &[BuiltinException]| {
            let classes: Vec<_> = classes
                .iter()
                .copied()
                .map(ExceptionClass::Builtin)
                .collect();
            super::common_base(&classes)
        };
        let cases: [(&[BuiltinException], BuiltinException); 5] = [
            (
                &[BuiltinException::KeyError, BuiltinException::IndexError],
                BuiltinException::LookupError,
            ),
            (
                &[
                    BuiltinException::KeyError,
                    BuiltinException::ZeroDivisionError,
                ],
                BuiltinException::Exception,
            ),
            (
                &[BuiltinException::ValueError],
                BuiltinException::ValueError,
            ),
            (
                &[BuiltinException::UnicodeError, BuiltinException::ValueError],
                BuiltinException::ValueError,
            ),
            (&[], BuiltinException::BaseException),
        ];
        for (classes, expected) in cases {
            assert_eq!(
                common(classes),
                ExceptionClass::Builtin(expected),
                "{classes:?}"
            );
        }
    }

    /// Returns a class of the program derived from `ancestors`, nearest first, then `builtin`.
    fn program_class(id: u32, ancestors: &[u32], builtin: BuiltinException) -> ExceptionClass {
        ExceptionClass::program(
            crate::typed::ClassId(id),
            format!("C{id}"),
            ancestors
                .iter()
                .map(|each| (crate::typed::ClassId(*each), format!("C{each}")))
                .collect(),
            builtin,
        )
    }

    #[test]
    fn a_program_class_derives_from_its_ancestors_and_their_built_in_bases() {
        let app = program_class(0, &[], BuiltinException::ValueError);
        let missing = program_class(1, &[0], BuiltinException::ValueError);
        let other = program_class(2, &[], BuiltinException::KeyError);
        assert!(missing.derives_from(&app));
        assert!(missing.derives_from(&missing));
        assert!(missing.derives_from(&ExceptionClass::Builtin(BuiltinException::Exception)));
        assert!(!app.derives_from(&missing));
        assert!(!other.derives_from(&app));
        assert!(!ExceptionClass::Builtin(BuiltinException::ValueError).derives_from(&app));
        assert_eq!(missing.base(), Some(app.clone()));
        assert_eq!(
            app.base(),
            Some(ExceptionClass::Builtin(BuiltinException::ValueError))
        );
        assert_eq!(super::common_base(&[missing.clone(), app.clone()]), app);
        assert_eq!(
            super::common_base(&[missing, other]),
            ExceptionClass::Builtin(BuiltinException::Exception)
        );
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
