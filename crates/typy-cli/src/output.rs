//! Writes diagnostics to standard error, and the output of `--emit` to a file or standard output.

use std::io::Write as _;

use camino::Utf8Path;
use typy_diagnostics::{
    Diagnostic, DiagnosticCode, RuleId, SourceLookup, render_json, render_text,
};
use typy_driver::{Emission, Failure, Sources};

use crate::exit_status::ExitStatus;

/// How diagnostics are written to standard error (DIA-05).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, clap::ValueEnum)]
pub(crate) enum ErrorFormat {
    /// Text for a person, a blank line between two diagnostics.
    #[default]
    Text,
    /// One JSON object per line, for an editor or CI.
    Json,
}

/// Writes diagnostics in the format the command line chose.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Reporter {
    format: ErrorFormat,
}

impl Reporter {
    /// Returns a reporter that writes in `format`.
    pub(crate) const fn new(format: ErrorFormat) -> Self {
        Self { format }
    }

    /// Writes rendered diagnostics to standard error.
    fn write(self, diagnostics: &[Diagnostic], sources: &dyn SourceLookup) {
        let text = match self.format {
            ErrorFormat::Text => diagnostics
                .iter()
                .map(|diagnostic| render_text(diagnostic, sources))
                .collect::<Vec<String>>()
                .join("\n"),
            ErrorFormat::Json => diagnostics
                .iter()
                .map(|diagnostic| render_json(diagnostic, sources))
                .collect(),
        };
        let mut error = std::io::stderr().lock();
        let _ = error.write_all(text.as_bytes());
    }

    /// Reports a failure and returns its status: `Internal` when any diagnostic is an
    /// `E9xxx` internal error, `Usage` when `typy.toml` is not valid (E0107, UC-11),
    /// `ProgramErrors` otherwise.
    pub(crate) fn report(self, failure: Failure) -> ExitStatus {
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
        self.write(&sorted, &failure.sources);
        status
    }

    /// Reports an internal error with no place in the source (NFR-09).
    pub(crate) fn internal(self, message: &str) {
        let diagnostic = Diagnostic::without_location(DiagnosticCode::error(9001), message)
            .with_rule(RuleId::NFR_09)
            .with_help(
                "this is a defect in TyPy; please report it with the program that caused it",
            );
        self.write(&[diagnostic], &Sources::None);
    }

    /// Reports a file that could not be written.
    pub(crate) fn unwritable(self, path: &Utf8Path, error: &std::io::Error) -> ExitStatus {
        let diagnostic = Diagnostic::without_location(
            DiagnosticCode::error(106),
            format!("cannot write {path}: {error}"),
        )
        .with_help("check that the directory exists and is writable");
        self.write(&[diagnostic], &Sources::None);
        ExitStatus::ProgramErrors
    }

    /// Writes the output of `--emit` to `output`, or to standard output when it is absent.
    pub(crate) fn emission(self, emission: &Emission, output: Option<&Utf8Path>) -> ExitStatus {
        let Some(path) = output else {
            let mut stdout = std::io::stdout().lock();
            return match stdout
                .write_all(emission.as_bytes())
                .and_then(|()| stdout.flush())
            {
                Ok(()) => ExitStatus::Success,
                Err(error) => self.unwritable(Utf8Path::new("standard output"), &error),
            };
        };
        match std::fs::write(path, emission.as_bytes()) {
            Ok(()) => ExitStatus::Success,
            Err(error) => self.unwritable(path, &error),
        }
    }
}

#[cfg(test)]
mod tests {
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
}
