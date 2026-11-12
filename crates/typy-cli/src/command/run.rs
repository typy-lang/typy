//! `typy run FILE [-- ARGS]` (CLI-02, UC-03): builds into the cache, then runs the program.

use std::process::Command;

use camino::Utf8PathBuf;
use typy_build::{BuildMode, BuildRequest};
use typy_driver::Pipeline;

use crate::cache;
use crate::exit_status::ExitStatus;
use crate::output::Reporter;

/// Builds the program unless the cache holds this build, then runs it with `arguments` and
/// returns its exit status.
pub(super) fn run(
    pipeline: &Pipeline,
    file: Utf8PathBuf,
    mode: BuildMode,
    overrides: typy_source::SettingOverrides,
    deny_warnings: bool,
    arguments: &[String],
    reporter: Reporter,
) -> ExitStatus {
    let request = match BuildRequest::new(file) {
        Ok(request) => request.with_mode(mode),
        Err(error) => return super::usage(&error),
    };
    let Some(files) = pipeline.sources(request.main()) else {
        // Checking loads the program again and reports why it does not load.
        return match pipeline.check(request.main()) {
            Ok(_) => ExitStatus::ProgramErrors,
            Err(failure) => reporter.report(failure),
        };
    };
    let name = request.main().file_stem().unwrap_or("program").to_owned();
    let executable = cache::directory()
        .join(cache::key(
            &request,
            &files,
            overrides,
            deny_warnings,
            &cache::compiler_identity(),
        ))
        .join(name);
    if !executable.exists() {
        if let Some(parent) = executable.parent()
            && let Err(error) = std::fs::create_dir_all(parent)
        {
            return reporter.unwritable(parent, &error);
        }
        match pipeline.run(&request.with_output(executable.clone())) {
            Ok(success) => reporter.warnings(success.warnings),
            Err(failure) => return reporter.report(failure),
        }
    } else if let Ok(success) = pipeline.check(request.main()) {
        // A build from the cache shows the warnings its program has, as a new build does.
        reporter.warnings(success.warnings);
    }
    match Command::new(&executable).args(arguments).status() {
        Ok(status) => ExitStatus::Program(program_status(status)),
        Err(error) => {
            reporter.internal(&format!("cannot start {executable}: {error}"));
            ExitStatus::Internal
        }
    }
}

/// Returns the program's exit status, or 128 plus the signal that ended it, as a shell does.
fn program_status(status: std::process::ExitStatus) -> u8 {
    use std::os::unix::process::ExitStatusExt as _;
    let code = status
        .code()
        .or_else(|| status.signal().map(|signal| 128 + signal))
        .unwrap_or(1);
    u8::try_from(code).unwrap_or(1)
}
