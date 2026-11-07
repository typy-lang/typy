//! The result of one conformance test.

use std::fmt;

/// Whether a test passed, failed, or could not judge TyPy because the oracles disagree.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Verdict {
    /// TyPy did what the test expects.
    Pass,
    /// TyPy did something else, as the reason says.
    Fail(String),
    /// The test cannot judge TyPy: its oracle versions disagree, or the file is not a valid
    /// test (STD-03).
    Invalid(String),
}

impl fmt::Display for Verdict {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Pass => formatter.write_str("pass"),
            Self::Fail(reason) => write!(formatter, "FAIL: {reason}"),
            Self::Invalid(reason) => write!(formatter, "INVALID: {reason}"),
        }
    }
}
