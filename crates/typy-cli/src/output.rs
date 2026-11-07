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
/// internal error, `ProgramErrors` otherwise.
pub(crate) fn report(failure: Failure) -> ExitStatus {
    let internal = failure
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.code().to_string().starts_with("E9"));
    let sorted = failure.diagnostics.into_sorted(&failure.sources);
    write_diagnostics(&sorted, &failure.sources);
    if internal {
        ExitStatus::Internal
    } else {
        ExitStatus::ProgramErrors
    }
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
