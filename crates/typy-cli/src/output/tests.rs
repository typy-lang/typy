//! Tests of `output`.

use super::{ErrorFormat, Reporter};

fn report(failure: Failure) -> ExitStatus {
    Reporter::new(ErrorFormat::Text).report(failure)
}
use crate::exit_status::ExitStatus;
use typy_diagnostics::{Diagnostic, DiagnosticCode, Diagnostics};
use typy_driver::Failure;

fn failure(code: DiagnosticCode) -> Failure {
    Failure::without_sources(Diagnostics::from(Diagnostic::without_location(code, "x")))
}

#[test]
fn an_internal_error_exits_with_101_and_any_other_with_1() {
    assert_eq!(
        report(failure(DiagnosticCode::error(9001))),
        ExitStatus::Internal
    );
    assert_eq!(
        report(failure(DiagnosticCode::error(702))),
        ExitStatus::ProgramErrors
    );
    assert_eq!(
        report(failure(DiagnosticCode::error(107))),
        ExitStatus::Usage
    );
}
