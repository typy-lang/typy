//! `typy NAME ARGS` for a NAME that is not a command of `typy`: runs the program `typy-NAME`
//! found on `PATH`, as `git` and `cargo` run theirs (CLI-32, ADR-0105).

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::exit_status::ExitStatus;

/// Runs `typy-NAME` with the remaining arguments, its streams and its exit status passed
/// through, and `TYPY` set to this compiler; an unknown command is wrong usage.
pub(super) fn external(arguments: &[String]) -> ExitStatus {
    let Some((name, rest)) = arguments.split_first() else {
        return ExitStatus::Usage;
    };
    // A command is a word, so it names a program on `PATH` and never a path.
    if !is_command_name(name) {
        eprintln!("error: `{name}` is not a command of typy");
        return ExitStatus::Usage;
    }
    let program = format!("typy-{name}");
    let Some(path) = std::env::var_os("PATH").and_then(|path| find(&program, &path)) else {
        eprintln!("error: `{name}` is not a command of typy, and no `{program}` was found on PATH");
        return ExitStatus::Usage;
    };
    let mut command = Command::new(&path);
    command.args(rest);
    if let Ok(compiler) = std::env::current_exe() {
        command.env("TYPY", compiler);
    }
    match command.status() {
        Ok(status) => ExitStatus::Program(super::run::program_status(status)),
        Err(error) => {
            eprintln!("error: cannot run {}: {error}", path.display());
            ExitStatus::Usage
        }
    }
}

/// Returns whether `name` may name an external command: letters, digits, `-` and `_`, not
/// starting with `-`.
fn is_command_name(name: &str) -> bool {
    !name.is_empty()
        && !name.starts_with('-')
        && name
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_'))
}

/// Returns the first executable file named `program` in the directories of `path`.
fn find(program: &str, path: &std::ffi::OsStr) -> Option<PathBuf> {
    std::env::split_paths(path)
        .map(|directory| directory.join(program))
        .find(|candidate| is_executable(candidate))
}

/// Returns whether `path` is a file its owner, group or others may run.
#[cfg(unix)]
fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt as _;
    path.metadata()
        .is_ok_and(|metadata| metadata.is_file() && metadata.permissions().mode() & 0o111 != 0)
}

/// Returns whether `path` is a file.
#[cfg(not(unix))]
fn is_executable(path: &Path) -> bool {
    path.is_file()
}

#[cfg(test)]
mod tests;
