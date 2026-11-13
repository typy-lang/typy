//! Runs the command the arguments name.

mod build;
mod check;
mod explain;
mod run;

use camino::Utf8PathBuf;
use typy_driver::Pipeline;

use crate::arguments::Command;
use crate::exit_status::ExitStatus;
use crate::output::Reporter;

/// Runs the command and returns the status `typy` exits with.
pub(crate) fn run(command: Command, deny_warnings: bool, reporter: Reporter) -> ExitStatus {
    let pipeline = match compiler() {
        Ok(path) => Pipeline::new(path).with_deny_warnings(deny_warnings),
        Err(message) => {
            reporter.internal(&message);
            return ExitStatus::Internal;
        }
    };
    match command {
        Command::Explain { code } => explain::explain(&code),
        Command::Check { file, settings } => check::check(
            &pipeline.with_overrides(settings.overrides()),
            &file,
            reporter,
        ),
        Command::Build {
            file,
            emit,
            output,
            mode,
            settings,
        } => build::build(
            &pipeline.with_overrides(settings.overrides()),
            file,
            emit,
            output.as_deref(),
            mode.build_mode(),
            reporter,
        ),
        Command::Run {
            file,
            mode,
            settings,
            arguments,
        } => run::run(
            &pipeline.with_overrides(settings.overrides()),
            file,
            mode.build_mode(),
            settings.overrides(),
            deny_warnings,
            &arguments,
            reporter,
        ),
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
