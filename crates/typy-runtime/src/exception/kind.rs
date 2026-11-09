//! The exception classes the runtime raises (EXC-02).

/// An exception's class.
#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(
    missing_docs,
    reason = "each variant is the Python exception class of the same name"
)]
pub enum ExceptionKind {
    IndexError = 1,
    OverflowError = 2,
    RecursionError = 3,
    TypeError = 4,
    ValueError = 5,
    ZeroDivisionError = 6,
    UnicodeEncodeError = 7,
    UnicodeDecodeError = 8,
    KeyError = 9,
    RuntimeError = 10,
}

impl ExceptionKind {
    /// Returns the class name Python prints.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::IndexError => "IndexError",
            Self::OverflowError => "OverflowError",
            Self::RecursionError => "RecursionError",
            Self::TypeError => "TypeError",
            Self::ValueError => "ValueError",
            Self::ZeroDivisionError => "ZeroDivisionError",
            Self::UnicodeEncodeError => "UnicodeEncodeError",
            Self::UnicodeDecodeError => "UnicodeDecodeError",
            Self::KeyError => "KeyError",
            Self::RuntimeError => "RuntimeError",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ExceptionKind;

    #[test]
    fn kinds_carry_their_python_names() {
        assert_eq!(ExceptionKind::ZeroDivisionError.name(), "ZeroDivisionError");
        assert_eq!(ExceptionKind::RecursionError.name(), "RecursionError");
    }
}
