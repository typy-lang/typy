//! Diagnostic codes such as `E0401` and `W0501`, and their severity.

use std::fmt;

/// Whether a diagnostic stops the build.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Severity {
    /// Stops the build; written `error`.
    Error,
    /// Reported without stopping the build, unless `--deny-warnings` is given (DIA-07).
    Warning,
}

impl Severity {
    /// Returns the word that starts a rendered diagnostic.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Error => "error",
            Self::Warning => "warning",
        }
    }
}

/// A stable diagnostic code: a severity letter and a number from 0 to 9999.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DiagnosticCode {
    severity: Severity,
    number: u16,
}

impl DiagnosticCode {
    const MAX_NUMBER: u16 = 9999;

    /// Returns the error code with this number.
    ///
    /// # Panics
    ///
    /// Panics when `number` is over 9999. Codes are constants, so this is a
    /// compile error in practice.
    #[must_use]
    pub const fn error(number: u16) -> Self {
        assert!(
            number <= Self::MAX_NUMBER,
            "a diagnostic code has four digits"
        );
        Self {
            severity: Severity::Error,
            number,
        }
    }

    /// Returns the warning code with this number.
    ///
    /// # Panics
    ///
    /// Panics when `number` is over 9999. Codes are constants, so this is a
    /// compile error in practice.
    #[must_use]
    pub const fn warning(number: u16) -> Self {
        assert!(
            number <= Self::MAX_NUMBER,
            "a diagnostic code has four digits"
        );
        Self {
            severity: Severity::Warning,
            number,
        }
    }

    /// Parses text of the form `E0401` or `W0501`.
    pub fn parse(text: &str) -> Result<Self, CodeError> {
        let malformed = || CodeError::Malformed {
            text: text.to_owned(),
        };
        let bytes = text.as_bytes();
        if bytes.len() != 5 || !bytes[1..].iter().all(u8::is_ascii_digit) {
            return Err(malformed());
        }
        let severity = match bytes[0] {
            b'E' => Severity::Error,
            b'W' => Severity::Warning,
            _ => return Err(malformed()),
        };
        let number = text[1..].parse().map_err(|_| malformed())?;
        Ok(Self { severity, number })
    }

    /// Returns the severity the code's letter gives.
    #[must_use]
    pub const fn severity(self) -> Severity {
        self.severity
    }

    /// Returns the number after the letter.
    #[must_use]
    pub const fn number(self) -> u16 {
        self.number
    }
}

impl fmt::Display for DiagnosticCode {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let letter = match self.severity {
            Severity::Error => 'E',
            Severity::Warning => 'W',
        };
        write!(formatter, "{letter}{:04}", self.number)
    }
}

/// The reason a text is not a diagnostic code.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CodeError {
    /// The text is not a letter `E` or `W` followed by four digits.
    Malformed {
        /// The text that was given.
        text: String,
    },
}

impl fmt::Display for CodeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Malformed { text } => write!(
                formatter,
                "`{text}` is not a diagnostic code; a code is E or W followed by four digits"
            ),
        }
    }
}

impl std::error::Error for CodeError {}

#[cfg(test)]
mod tests {
    use super::{CodeError, DiagnosticCode, Severity};
    use proptest::prelude::*;

    #[test]
    fn codes_are_written_with_their_letter_and_four_digits() {
        assert_eq!(DiagnosticCode::error(401).to_string(), "E0401");
        assert_eq!(DiagnosticCode::warning(7).to_string(), "W0007");
        assert_eq!(DiagnosticCode::error(9999).to_string(), "E9999");
    }

    #[test]
    fn well_formed_codes_parse_with_their_severity() {
        let code = DiagnosticCode::parse("W0501").unwrap();
        assert_eq!(code.severity(), Severity::Warning);
        assert_eq!(code.number(), 501);
        assert_eq!(
            DiagnosticCode::parse("E0000").unwrap(),
            DiagnosticCode::error(0)
        );
    }

    #[test]
    fn malformed_codes_are_refused_with_the_text() {
        for text in [
            "",
            "E",
            "E401",
            "E04011",
            "e0401",
            "X0401",
            "E04a1",
            "E 401",
            "E-401",
            "\u{e9}0401",
        ] {
            assert_eq!(
                DiagnosticCode::parse(text),
                Err(CodeError::Malformed {
                    text: text.to_owned()
                }),
                "{text:?}"
            );
        }
    }

    #[test]
    fn code_error_message_names_the_text() {
        let error = DiagnosticCode::parse("Q1").unwrap_err();
        assert!(error.to_string().contains("`Q1`"));
    }

    #[test]
    fn severity_labels_are_lower_case_words() {
        assert_eq!(Severity::Error.label(), "error");
        assert_eq!(Severity::Warning.label(), "warning");
    }

    proptest! {
        #[test]
        fn display_and_parse_round_trip(number in 0_u16..=9999, warning in any::<bool>()) {
            let code = if warning {
                DiagnosticCode::warning(number)
            } else {
                DiagnosticCode::error(number)
            };
            prop_assert_eq!(DiagnosticCode::parse(&code.to_string()), Ok(code));
        }

        #[test]
        fn parse_never_panics(text in "\\PC{0,8}") {
            let _ = DiagnosticCode::parse(&text);
        }
    }
}
