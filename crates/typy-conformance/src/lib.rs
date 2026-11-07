//! The conformance suite: Python programs run under TyPy and under each CPython oracle, with
//! their results compared (UC-09, STD-02, NFR-10).
#![forbid(unsafe_code)]

mod expectation;
mod observation;
mod oracle;
mod subject;
mod suite;
mod test_case;
mod verdict;

pub use expectation::Expectation;
pub use observation::Observation;
pub use suite::{Coverage, Report, Suite};
pub use test_case::{CaseError, TestCase};
pub use verdict::Verdict;
