//! Judges TyPy on one test: builds and runs an accepted program, or checks a rejected one.

use std::process::Command;

use camino::{Utf8Path, Utf8PathBuf};
use typy_build::{BuildMode, BuildRequest, EmitStage, RuntimeArchive};
use typy_diagnostics::{Diagnostic, DiagnosticCode, SourceLookup};
use typy_driver::{Emission, Failure, Outcome, Pipeline};

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

/// Builds the program's object file in debug mode, links it with the runtime built with
/// AddressSanitizer and with GCC's AddressSanitizer library, and compares the run with the
/// oracles. Every allocation and free is checked, leaks are reported at exit, and every memory
/// access of the runtime is checked; generated code is not instrumented (OI-16).
pub(crate) fn judge_sanitized(
    pipeline: &Pipeline,
    compiler: &Utf8Path,
    case: &TestCase,
    expected: &Observation,
) -> Verdict {
    let Some(archive) = instrumented_archive(compiler) else {
        return Verdict::Invalid(
            "the instrumented runtime archive is missing; run `make runtime`".to_owned(),
        );
    };
    let Ok(directory) = tempfile::tempdir() else {
        return Verdict::Invalid("cannot make a temporary directory".to_owned());
    };
    let Ok(root) = Utf8PathBuf::from_path_buf(directory.path().to_owned()) else {
        return Verdict::Invalid("the temporary directory's path is not UTF-8".to_owned());
    };
    let request = match BuildRequest::new(case.path.clone()) {
        Ok(request) => request.with_emit(EmitStage::Obj),
        Err(error) => return Verdict::Invalid(error.to_string()),
    };
    let object = match pipeline.run(&request) {
        Ok(Outcome::Emitted(Emission::Bytes(bytes))) => bytes,
        Ok(_) => return Verdict::Fail("no object file was emitted".to_owned()),
        Err(failure) => {
            return Verdict::Fail(format!(
                "TyPy rejected the program: {}",
                describe(&failure).join("; ")
            ));
        }
    };
    let object_path = root.join("program.o");
    let executable = root.join("sanitized");
    if std::fs::write(&object_path, object).is_err() {
        return Verdict::Invalid("cannot write the object file".to_owned());
    }
    let linked = Observation::of(Command::new("cc").args([
        "-fsanitize=address",
        "-o",
        executable.as_str(),
        object_path.as_str(),
        archive.as_str(),
        "-lm",
    ]));
    match linked {
        Ok(link) if link.status == Some(0) => {}
        Ok(link) => return Verdict::Invalid(format!("the sanitized link failed: {link}")),
        Err(reason) => return Verdict::Invalid(format!("the sanitized link {reason}")),
    }
    // A sanitizer report ends the program with status 86 and its summary as the last line.
    let observed = Observation::of(
        Command::new(&executable).env("ASAN_OPTIONS", "detect_leaks=1:exitcode=86"),
    );
    match observed {
        Ok(observed) if observed == *expected => Verdict::Pass,
        Ok(observed) => Verdict::Fail(format!(
            "the sanitized build shows {observed}; CPython shows {expected}"
        )),
        Err(reason) => Verdict::Fail(format!("the sanitized build {reason}")),
    }
}

/// Returns the runtime archive built with AddressSanitizer, which `make runtime` stages in
/// `<prefix>/lib/typy-asan/<triple>/` beside the plain one, once its version is checked.
fn instrumented_archive(compiler: &Utf8Path) -> Option<Utf8PathBuf> {
    let prefix = compiler.parent()?.parent()?;
    let path = prefix
        .join("lib/typy-asan")
        .join(typy_codegen::Target::host().triple.as_str())
        .join("libtypy_runtime.a");
    RuntimeArchive::open(&path)
        .ok()
        .map(|archive| archive.path().to_owned())
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
