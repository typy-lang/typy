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
    /// The run does not judge the test, for the reason given: its output depends on the
    /// architecture, and the run builds for another than the oracle's (NFR-16).
    Skipped(String),
}

impl fmt::Display for Verdict {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Pass => formatter.write_str("pass"),
            Self::Fail(reason) => write!(formatter, "FAIL: {reason}"),
            Self::Invalid(reason) => write!(formatter, "INVALID: {reason}"),
            Self::Skipped(reason) => write!(formatter, "skipped: {reason}"),
        }
    }
}
