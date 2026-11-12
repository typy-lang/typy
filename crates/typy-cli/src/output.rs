//! Writes diagnostics to standard error, and the output of `--emit` to a file or standard output.

use std::io::Write as _;

use camino::Utf8Path;
use typy_diagnostics::{Diagnostic, DiagnosticCode, RuleId, SourceLookup, render_text};
use typy_driver::{Emission, Failure, Sources};

use crate::exit_status::ExitStatus;

/// Writes rendered diagnostics to standard error, a blank line between two of them.
fn write_diagnostics(diagnostics: &[Diagnostic], sources: &dyn SourceLookup) {
    let text: Vec<String> = diagnostics
        .iter()
        .map(|diagnostic| render_text(diagnostic, sources))
        .collect();
    let mut error = std::io::stderr().lock();
    let _ = error.write_all(text.join("\n").as_bytes());
}

/// Reports a failure and returns its status: `Internal` when any diagnostic is an `E9xxx`
/// internal error, `Usage` when `typy.toml` is not valid (E0107, UC-11), `ProgramErrors`
/// otherwise.
pub(crate) fn report(failure: Failure) -> ExitStatus {
    let has = |prefix: &str| {
        failure
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code().to_string().starts_with(prefix))
    };
    let status = if has("E9") {
        ExitStatus::Internal
    } else if has("E0107") {
        ExitStatus::Usage
    } else {
        ExitStatus::ProgramErrors
    };
    let sorted = failure.diagnostics.into_sorted(&failure.sources);
    write_diagnostics(&sorted, &failure.sources);
    status
}

/// Reports an internal error with no place in the source (NFR-09).
pub(crate) fn report_internal(message: &str) {
    let diagnostic = Diagnostic::without_location(DiagnosticCode::error(9001), message)
        .with_rule(RuleId::NFR_09)
        .with_help("this is a defect in TyPy; please report it with the program that caused it");
    write_diagnostics(&[diagnostic], &Sources::None);
}

/// Reports a file that could not be written.
pub(crate) fn report_unwritable(path: &Utf8Path, error: &std::io::Error) -> ExitStatus {
    let diagnostic = Diagnostic::without_location(
        DiagnosticCode::error(106),
        format!("cannot write {path}: {error}"),
    )
    .with_help("check that the directory exists and is writable");
    write_diagnostics(&[diagnostic], &Sources::None);
    ExitStatus::ProgramErrors
}

/// Writes the output of `--emit` to `output`, or to standard output when it is absent.
pub(crate) fn write_emission(emission: &Emission, output: Option<&Utf8Path>) -> ExitStatus {
    let Some(path) = output else {
        let mut stdout = std::io::stdout().lock();
        return match stdout
            .write_all(emission.as_bytes())
            .and_then(|()| stdout.flush())
        {
            Ok(()) => ExitStatus::Success,
            Err(error) => report_unwritable(Utf8Path::new("standard output"), &error),
        };
    };
    match std::fs::write(path, emission.as_bytes()) {
        Ok(()) => ExitStatus::Success,
        Err(error) => report_unwritable(path, &error),
    }
}

#[cfg(test)]
mod tests {
    use super::report;
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
}
