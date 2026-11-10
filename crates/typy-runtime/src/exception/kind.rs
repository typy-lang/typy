//! The exception classes the runtime raises itself (EXC-02), each named by its class.

use super::classes;
use crate::type_object::TypeObject;

/// An exception class the runtime raises.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(
    missing_docs,
    reason = "each variant is the Python exception class of the same name"
)]
pub enum ExceptionKind {
    AssertionError,
    IndexError,
    KeyError,
    OverflowError,
    RecursionError,
    RuntimeError,
    StopIteration,
    TypeError,
    UnicodeDecodeError,
    UnicodeEncodeError,
    ValueError,
    ZeroDivisionError,
}

impl ExceptionKind {
    /// Every kind, in the order of the variants.
    #[cfg(test)]
    pub(crate) const ALL: [Self; 12] = [
        Self::AssertionError,
        Self::IndexError,
        Self::KeyError,
        Self::OverflowError,
        Self::RecursionError,
        Self::RuntimeError,
        Self::StopIteration,
        Self::TypeError,
        Self::UnicodeDecodeError,
        Self::UnicodeEncodeError,
        Self::ValueError,
        Self::ZeroDivisionError,
    ];

    /// Returns the class of the kind.
    #[must_use]
    pub fn class(self) -> &'static TypeObject {
        match self {
            Self::AssertionError => &classes::ASSERTION_ERROR,
            Self::IndexError => &classes::INDEX_ERROR,
            Self::KeyError => &classes::KEY_ERROR,
            Self::OverflowError => &classes::OVERFLOW_ERROR,
            Self::RecursionError => &classes::RECURSION_ERROR,
            Self::RuntimeError => &classes::RUNTIME_ERROR,
            Self::StopIteration => &classes::STOP_ITERATION,
            Self::TypeError => &classes::TYPE_ERROR,
            Self::UnicodeDecodeError => &classes::UNICODE_DECODE_ERROR,
            Self::UnicodeEncodeError => &classes::UNICODE_ENCODE_ERROR,
            Self::ValueError => &classes::VALUE_ERROR,
            Self::ZeroDivisionError => &classes::ZERO_DIVISION_ERROR,
        }
    }

    /// Returns the class name Python prints.
    #[must_use]
    pub fn name(self) -> &'static str {
        self.class().name()
    }
}

#[cfg(test)]
mod tests {
    use super::ExceptionKind;

    #[test]
    fn kinds_carry_their_python_names() {
        assert_eq!(ExceptionKind::ZeroDivisionError.name(), "ZeroDivisionError");
        assert_eq!(ExceptionKind::RecursionError.name(), "RecursionError");
        assert_eq!(ExceptionKind::StopIteration.name(), "StopIteration");
    }

    #[test]
    fn each_kind_has_its_own_class() {
        for (index, kind) in ExceptionKind::ALL.iter().enumerate() {
            for other in &ExceptionKind::ALL[index + 1..] {
                assert!(!core::ptr::eq(kind.class(), other.class()), "{kind:?}");
            }
        }
    }
}
