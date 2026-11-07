//! `typy check FILE` (CLI-03, UC-01).

use camino::Utf8Path;
use typy_driver::Pipeline;

use crate::exit_status::ExitStatus;
use crate::output::report;

/// Checks the program and reports every diagnostic; nothing is written.
pub(super) fn check(pipeline: &Pipeline, file: &Utf8Path) -> ExitStatus {
    match pipeline.check(file) {
        Ok(_) => ExitStatus::Success,
        Err(failure) => report(failure),
    }
}
