//! Runs the command the arguments name.

mod build;
mod check;
mod explain;
mod external;
mod generation;
mod print;
mod run;

use camino::Utf8PathBuf;
use typy_driver::Pipeline;

use crate::arguments::{Command, Global};
use crate::exit_status::ExitStatus;
use crate::output::Reporter;

/// Runs the command and returns the status `typy` exits with.
pub(crate) fn run(command: Command, global: Global, reporter: Reporter) -> ExitStatus {
    // An external command and the processor list need no pipeline.
    let command = match command {
        Command::External(arguments) => return external::external(&arguments),
        Command::ProcessorHelp { target } => return print::processor_help(target, reporter),
        other => other,
    };
    let pipeline = match compiler() {
        Ok(path) => Pipeline::new(path)
            .with_deny_warnings(global.deny_warnings)
            .with_verbose(global.verbose),
        Err(message) => {
            reporter.internal(&message);
            return ExitStatus::Internal;
        }
    };
    let (status, pipeline) = dispatch(command, pipeline, global.deny_warnings, reporter);
    if global.timings {
        write_timings(&pipeline.timings());
    }
    status
}

/// Runs a command of the compiler with `pipeline`, configured for it, and returns its status
/// with the pipeline, which holds the time each stage took.
fn dispatch(
    command: Command,
    pipeline: Pipeline,
    deny_warnings: bool,
    reporter: Reporter,
) -> (ExitStatus, Pipeline) {
    match command {
        Command::Explain { code } => (explain::explain(&code), pipeline),
        // The target's operating system gives `sys.platform` (LIB-22).
        Command::Check {
            file,
            settings,
            library,
            target,
        } => {
            let pipeline = pipeline.with_overrides(settings.overrides_for(library));
            let target = target
                .triple
                .map_or_else(typy_codegen::Target::host, |triple| typy_codegen::Target {
                    triple,
                });
            (check::check(&pipeline, &file, target, reporter), pipeline)
        }
        Command::Build {
            file,
            emit,
            output,
            generation,
            temps,
            settings,
            library,
            target,
        } => {
            let triple = target.triple.unwrap_or(typy_codegen::Target::host().triple);
            let generation = match resolve(&generation, triple) {
                Ok(resolved) => resolved,
                Err(status) => return (status, pipeline),
            };
            let pipeline = pipeline
                .with_overrides(settings.overrides_for(library))
                .with_save_temps(temps.save_temps);
            let flags = build::Flags {
                emit,
                output,
                generation,
                library: library.kind,
                target: target.triple,
            };
            (build::build(&pipeline, file, &flags, reporter), pipeline)
        }
        Command::Run {
            file,
            generation,
            temps,
            settings,
            arguments,
        } => {
            let generation = match resolve(&generation, typy_codegen::Target::host().triple) {
                Ok(resolved) => resolved,
                Err(status) => return (status, pipeline),
            };
            let pipeline = pipeline
                .with_overrides(settings.overrides())
                .with_save_temps(temps.save_temps);
            let status = run::run(
                &pipeline,
                file,
                &generation,
                settings.overrides(),
                deny_warnings,
                &arguments,
                reporter,
            );
            (status, pipeline)
        }
        Command::Print {
            what,
            file,
            settings,
            library,
            target,
        } => {
            let status = print::print(
                what,
                file.as_deref(),
                settings.overrides_for(library),
                target.triple,
                reporter,
            );
            (status, pipeline)
        }
        Command::External(_) | Command::ProcessorHelp { .. } => {
            unreachable!("taken before the pipeline is made")
        }
    }
}

/// Returns how a build for `triple` generates code, or wrong usage, said on standard error,
/// when the flags conflict or name a processor the target does not have.
fn resolve(
    flags: &crate::arguments::Generation,
    triple: typy_codegen::TargetTriple,
) -> Result<generation::Generation, ExitStatus> {
    flags
        .choice()
        .and_then(|choice| generation::resolve(choice, triple))
        .map_err(|message| {
            eprintln!("error: {message}");
            ExitStatus::Usage
        })
}

/// Writes how long each stage took, then the total, to standard error (CLI-22).
fn write_timings(timings: &[(&'static str, std::time::Duration)]) {
    let total: std::time::Duration = timings.iter().map(|(_, time)| *time).sum();
    for (stage, time) in timings.iter().chain(std::iter::once(&("total", total))) {
        eprintln!("{stage:<9}{:>9.3} s", time.as_secs_f64());
    }
}

/// Returns this executable's path, which locates the runtime archives.
fn compiler() -> Result<Utf8PathBuf, String> {
    let path = std::env::current_exe()
        .map_err(|error| format!("cannot find the typy executable: {error}"))?;
    Utf8PathBuf::from_path_buf(path).map_err(|path| {
        format!(
            "the typy executable's path is not UTF-8: {}",
            path.display()
        )
    })
}

/// Reports a main file that is not a `.py` file as wrong usage.
fn usage(error: &typy_build::RequestError) -> ExitStatus {
    eprintln!("error: {error}");
    ExitStatus::Usage
}
