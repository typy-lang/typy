//! Runs the programs of the C toolchain as subprocesses, each with an argument list and no
//! shell, and names the diagnostics of their failures (A-01).

use std::io;
use std::process::Command;

use camino::{Utf8Path, Utf8PathBuf};
use typy_diagnostics::{Diagnostic, DiagnosticCode, Diagnostics, RuleId};

fn link_failed(message: String, help: String) -> Diagnostics {
    Diagnostics::from(
        Diagnostic::without_location(DiagnosticCode::error(702), message)
            .with_rule(RuleId::A_01)
            .with_help(help),
    )
}

/// Runs `program` with `arguments`: E0701 when it is not installed, and E0702 with what it
/// wrote when it fails.
pub(crate) fn run_tool(program: &str, arguments: &[String]) -> Result<(), Diagnostics> {
    let run = match Command::new(program).args(arguments).output() {
        Ok(run) => run,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            return Err(Diagnostics::from(
                Diagnostic::without_location(
                    DiagnosticCode::error(701),
                    format!("the C toolchain program `{program}` was not found"),
                )
                .with_rule(RuleId::A_01)
                .with_help(format!(
                    "install `{program}`, which TyPy runs to link the program"
                )),
            ));
        }
        Err(error) => {
            return Err(link_failed(
                format!("cannot run `{program}`: {error}"),
                format!("the command was `{program} {}`", arguments.join(" ")),
            ));
        }
    };
    if run.status.success() {
        return Ok(());
    }
    Err(link_failed(
        format!("`{program}` failed with {}", run.status),
        format!(
            "the command was `{program} {}`; it wrote:\n{}",
            arguments.join(" "),
            String::from_utf8_lossy(&run.stderr).trim_end()
        ),
    ))
}

/// Returns the directory an output goes in, where its temporary files go too, so the
/// finished file is renamed into place.
pub(crate) fn output_directory(output: &Utf8Path) -> Utf8PathBuf {
    match output.parent() {
        Some(parent) if !parent.as_str().is_empty() => parent.to_owned(),
        _ => Utf8PathBuf::from("."),
    }
}

/// Returns the E0106 of a file in `directory` that cannot be written; `what` names it.
pub(crate) fn cannot_write(directory: &Utf8Path, what: &str) -> impl Fn(io::Error) -> Diagnostics {
    let directory = directory.to_owned();
    let what = what.to_owned();
    move |error: io::Error| {
        Diagnostics::from(
            Diagnostic::without_location(
                DiagnosticCode::error(106),
                format!("cannot write {what} in {directory}: {error}"),
            )
            .with_help("check that the directory exists and is writable"),
        )
    }
}

/// Temporary paths are made from a UTF-8 directory and an ASCII name, so they are UTF-8.
pub(crate) fn utf8(path: &std::path::Path) -> Utf8PathBuf {
    Utf8PathBuf::from(path.to_string_lossy().into_owned())
}
