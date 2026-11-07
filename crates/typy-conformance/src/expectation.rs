//! What a test expects of TyPy: that it accepts the program, or rejects it with one code at one
//! place.

use std::fmt;

use typy_diagnostics::DiagnosticCode;

/// The line `# typy-expect: ...` of a test.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Expectation {
    /// TyPy builds the program, and it runs as it does under CPython.
    Accepted,
    /// `typy check` reports exactly this code at this line and column, and nothing else.
    Rejected {
        /// The code.
        code: DiagnosticCode,
        /// The line, from 1.
        line: u32,
        /// The column, from 1, in code points.
        column: u32,
    },
}

impl Expectation {
    /// Returns the expectation `accepted` or `rejected CODE at LINE:COLUMN` writes, or `None`.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        if text == "accepted" {
            return Some(Self::Accepted);
        }
        let rest = text.strip_prefix("rejected ")?;
        let (code, place) = rest.split_once(" at ")?;
        let (line, column) = place.split_once(':')?;
        Some(Self::Rejected {
            code: DiagnosticCode::parse(code).ok()?,
            line: line.parse().ok().filter(|line| *line > 0)?,
            column: column.parse().ok().filter(|column| *column > 0)?,
        })
    }
}

impl fmt::Display for Expectation {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Accepted => formatter.write_str("accepted"),
            Self::Rejected { code, line, column } => {
                write!(formatter, "rejected {code} at {line}:{column}")
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Expectation;

    #[test]
    fn both_forms_parse_and_print_back() {
        for text in ["accepted", "rejected E0302 at 4:12"] {
            assert_eq!(Expectation::parse(text).unwrap().to_string(), text);
        }
    }

    #[test]
    fn malformed_expectations_are_refused() {
        for text in [
            "",
            "accept",
            "rejected E0302",
            "rejected E0302 at 4",
            "rejected E302 at 4:12",
            "rejected E0302 at 0:1",
            "rejected E0302 at 1:x",
        ] {
            assert_eq!(Expectation::parse(text), None, "{text:?}");
        }
    }
}
