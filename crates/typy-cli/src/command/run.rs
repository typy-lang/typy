//! `typy run FILE [-- ARGS]`, `typy run -c CODE` and `typy run -m MODULE` (CLI-02, CLI-28,
//! UC-03): builds into the cache, then runs the program.

use std::os::unix::process::CommandExt as _;
use std::process::Command;

use typy_build::BuildRequest;
use typy_driver::Pipeline;
use typy_source::MainSource;

use crate::cache;
use crate::exit_status::ExitStatus;
use crate::output::Reporter;

/// Builds the program unless the cache holds this build, then runs it with `arguments` and
/// returns its exit status.
pub(super) fn run(
    pipeline: &Pipeline,
    main: MainSource,
    generation: &super::generation::Generation,
    overrides: typy_source::SettingOverrides,
    arguments: &[String],
    reporter: Reporter,
) -> ExitStatus {
    let request = match BuildRequest::of_main(main) {
        Ok(request) => request
            .with_mode(generation.mode)
            .with_processor(generation.processor.clone(), generation.features.clone()),
        Err(error) => return super::usage(&error),
    };
    let Some(files) = pipeline.sources(request.main()) else {
        // Checking loads the program again and reports why it does not load.
        return match pipeline.check(request.main(), request.target()) {
            Ok(_) => ExitStatus::ProgramErrors,
            Err(failure) => reporter.report(failure),
        };
    };
    let name = request.main().program_name().to_owned();
    let argument_zero = argument_zero(request.main(), &files);
    let executable = cache::directory()
        .join(cache::key(
            &request,
            &files,
            overrides,
            pipeline.warning_levels(),
            &cache::compiler_identity(&request),
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
    } else if let Ok(success) = pipeline.check(request.main(), request.target()) {
        // A build from the cache shows the warnings its program has, as a new build does.
        reporter.warnings(success.warnings);
    }
    if pipeline.is_verbose() {
        eprintln!(
            "{}",
            typy_build::command_line(executable.as_str(), arguments)
        );
    }
    let mut command = Command::new(&executable);
    if let Some(zero) = argument_zero {
        command.arg0(zero);
    }
    match command.args(arguments).status() {
        Ok(status) => ExitStatus::Program(program_status(status)),
        Err(error) => {
            reporter.internal(&format!("cannot start {executable}: {error}"));
            ExitStatus::Internal
        }
    }
}

/// Returns what the program reads as `sys.argv[0]` when it is not the executable's path: `-c`
/// for code, and the full path of the module's file for a module, as CPython gives them
/// (CLI-28). `files` start with the main file, by its path from the current directory.
fn argument_zero(main: &MainSource, files: &[(String, String)]) -> Option<String> {
    match main {
        MainSource::File(_) => None,
        MainSource::Code(_) => Some("-c".to_owned()),
        MainSource::Module(_) => {
            let path = files.first().map_or("", |(path, _)| path.as_str());
            let directory = std::env::current_dir().unwrap_or_default();
            Some(directory.join(path).to_string_lossy().into_owned())
        }
    }
}

/// Returns the program's exit status, or 128 plus the signal that ended it, as a shell does.
pub(super) fn program_status(status: std::process::ExitStatus) -> u8 {
    use std::os::unix::process::ExitStatusExt as _;
    let code = status
        .code()
        .or_else(|| status.signal().map(|signal| 128 + signal))
        .unwrap_or(1);
    u8::try_from(code).unwrap_or(1)
}
