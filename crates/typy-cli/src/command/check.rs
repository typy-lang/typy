//! `typy check FILE` (CLI-03, UC-01).

use camino::Utf8Path;
use typy_codegen::Target;
use typy_driver::Pipeline;

use crate::exit_status::ExitStatus;
use crate::output::Reporter;

/// Checks the program for `target` and reports every diagnostic; nothing is written.
pub(super) fn check(
    pipeline: &Pipeline,
    file: &Utf8Path,
    target: Target,
    reporter: Reporter,
) -> ExitStatus {
    match pipeline.check(file, target) {
        Ok(success) => {
            reporter.warnings(success.warnings);
            ExitStatus::Success
        }
        Err(failure) => reporter.report(failure),
    }
}
