//! Judges TyPy on one test: builds and runs an accepted program, or checks a rejected one.

use std::process::Command;

use camino::Utf8PathBuf;
use typy_build::{BuildMode, BuildRequest};
use typy_diagnostics::{Diagnostic, DiagnosticCode, SourceLookup};
use typy_driver::{Failure, Pipeline};

use crate::observation::Observation;
use crate::test_case::TestCase;
use crate::verdict::Verdict;

/// Returns `CODE at LINE:COLUMN: message` for each diagnostic of a failure.
fn describe(failure: &Failure) -> Vec<String> {
    failure
        .diagnostics
        .iter()
        .map(|diagnostic| match place(diagnostic, &failure.sources) {
            Some((line, column)) => format!(
                "{} at {line}:{column}: {}",
                diagnostic.code(),
                diagnostic.message()
            ),
            None => format!("{}: {}", diagnostic.code(), diagnostic.message()),
        })
        .collect()
}

fn place(diagnostic: &Diagnostic, sources: &dyn SourceLookup) -> Option<(u32, u32)> {
    let span = diagnostic.span()?;
    let position = sources.position(span.file(), span.start());
    Some((position.line, position.column))
}

/// Builds the program in debug and in release mode, runs each build, and compares it with what
/// the oracles showed (STD-02).
pub(crate) fn judge_accepted(
    pipeline: &Pipeline,
    case: &TestCase,
    expected: &Observation,
) -> Verdict {
    let Ok(directory) = tempfile::tempdir() else {
        return Verdict::Invalid("cannot make a temporary directory".to_owned());
    };
    let Ok(directory_path) = Utf8PathBuf::from_path_buf(directory.path().to_owned()) else {
        return Verdict::Invalid("the temporary directory's path is not UTF-8".to_owned());
    };
    for mode in [BuildMode::Debug, BuildMode::Release] {
        let request = match BuildRequest::new(case.path.clone()) {
            Ok(request) => request
                .with_mode(mode)
                .with_output(directory_path.join(mode.name())),
            Err(error) => return Verdict::Invalid(error.to_string()),
        };
        let executable = request.executable_path();
        if let Err(failure) = pipeline.run(&request) {
            return Verdict::Fail(format!(
                "TyPy rejected the program in {} mode: {}",
                mode.name(),
                describe(&failure).join("; ")
            ));
        }
        let observed = match Observation::of(&mut Command::new(&executable)) {
            Ok(observed) => observed,
            Err(reason) => return Verdict::Fail(format!("the {} build {reason}", mode.name())),
        };
        if observed != *expected {
            return Verdict::Fail(format!(
                "the {} build shows {observed}; CPython shows {expected}",
                mode.name()
            ));
        }
    }
    Verdict::Pass
}

/// Checks the program and passes when TyPy reports exactly `code` at `line:column`.
pub(crate) fn judge_rejected(
    pipeline: &Pipeline,
    case: &TestCase,
    code: DiagnosticCode,
    line: u32,
    column: u32,
) -> Verdict {
    let Err(failure) = pipeline.check(&case.path) else {
        return Verdict::Fail("TyPy accepted the program".to_owned());
    };
    let mut diagnostics = failure.diagnostics.iter();
    let only = diagnostics
        .next()
        .filter(|_| diagnostics.next().is_none())
        .filter(|diagnostic| {
            diagnostic.code() == code && place(diagnostic, &failure.sources) == Some((line, column))
        });
    match only {
        Some(_) => Verdict::Pass,
        None => Verdict::Fail(format!(
            "expected only {code} at {line}:{column}, got {}",
            describe(&failure).join("; ")
        )),
    }
}
