//! The `typy` command: `check`, `build` and `run` (CLI-01 to CLI-06).
#![forbid(unsafe_code)]

mod arguments;
mod cache;
mod command;
mod exit_status;
mod guard;
mod output;

use clap::Parser as _;

/// The stack the compiler runs on. Its passes recurse over the syntax tree, which nests up to
/// `typy_source::MAX_DEPTH` levels; the memory is reserved, and only what a program's nesting
/// uses is taken (NFR-09).
const COMPILER_STACK: usize = 512 * 1024 * 1024;

fn main() -> std::process::ExitCode {
    let arguments = arguments::Arguments::parse();
    let reporter = output::Reporter::new(arguments.error_format);
    let deny_warnings = arguments.deny_warnings;
    let compiler = std::thread::Builder::new()
        .name("compiler".to_owned())
        .stack_size(COMPILER_STACK)
        .spawn(move || {
            guard::guarded(reporter, || {
                command::run(arguments.command, deny_warnings, reporter)
            })
        });
    // The guard turns a panic into E9001 on the compiler's own thread, so joining it fails
    // only when the thread cannot start.
    if let Ok(Ok(status)) = compiler.map(std::thread::JoinHandle::join) {
        return status.into();
    }
    reporter.internal("the compiler's thread could not run");
    exit_status::ExitStatus::Internal.into()
}
