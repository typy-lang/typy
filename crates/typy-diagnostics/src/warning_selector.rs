//! The warnings one `-W`, `-A` or `-D` option names: one code, or every warning (CLI-27).

use std::fmt;

use crate::code::{DiagnosticCode, Severity};
use crate::registry::{CODES, CodeStatus, lookup};

/// The warnings one option names.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum WarningSelector {
    /// `warnings`: every warning.
    All,
    /// One warning code that the compiler reports, such as W0101.
    Code(DiagnosticCode),
}

impl WarningSelector {
    /// Returns the warnings `text` names: `warnings`, or the code of a warning the compiler
    /// reports.
    pub fn parse(text: &str) -> Result<Self, SelectorError> {
        if text == "warnings" {
            return Ok(Self::All);
        }
        let reported = DiagnosticCode::parse(text).ok().filter(|code| {
            code.severity() == Severity::Warning
                && lookup(*code).is_some_and(|info| info.status == CodeStatus::Active)
        });
        reported.map(Self::Code).ok_or_else(|| SelectorError {
            text: text.to_owned(),
        })
    }

    /// Returns true when the selector names `code`.
    #[must_use]
    pub fn names(self, code: DiagnosticCode) -> bool {
        match self {
            Self::All => code.severity() == Severity::Warning,
            Self::Code(named) => named == code,
        }
    }
}

/// The reason a text names no warning.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SelectorError {
    /// The text that was given.
    pub text: String,
}

impl fmt::Display for SelectorError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let codes: Vec<String> = CODES
            .iter()
            .filter(|info| {
                info.code.severity() == Severity::Warning && info.status == CodeStatus::Active
            })
            .map(|info| info.code.to_string())
            .collect();
        write!(
            formatter,
            "`{}` names no warning; give `warnings` or one of {}",
            self.text,
            codes.join(", ")
        )
    }
}

impl std::error::Error for SelectorError {}

#[cfg(test)]
mod tests;
