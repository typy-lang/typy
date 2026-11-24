//! Tests of `output`.

use super::{ErrorFormat, Reporter};

fn report(failure: Failure) -> ExitStatus {
    Reporter::new(ErrorFormat::Text, false).report(failure)
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

#[test]
fn auto_colours_only_a_terminal_without_no_color() {
    use clap::ColorChoice::{Always, Auto, Never};
    let set = Some(std::ffi::OsStr::new("1"));
    let empty = Some(std::ffi::OsStr::new(""));
    let cases = [
        (Always, false, set, true),
        (Never, true, None, false),
        (Auto, true, None, true),
        (Auto, true, empty, true),
        (Auto, true, set, false),
        (Auto, false, None, false),
    ];
    for (choice, terminal, no_color, expected) in cases {
        assert_eq!(
            super::colours(choice, terminal, no_color),
            expected,
            "{choice:?} {terminal} {no_color:?}"
        );
    }
}
