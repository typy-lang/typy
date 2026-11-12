//! The `typy` command: `check`, `build` and `run` (CLI-01 to CLI-06).
#![forbid(unsafe_code)]

mod arguments;
mod cache;
mod command;
mod exit_status;
mod guard;
mod output;

use clap::Parser as _;

fn main() -> std::process::ExitCode {
    let arguments = arguments::Arguments::parse();
    let reporter = output::Reporter::new(arguments.error_format);
    guard::guarded(reporter, || command::run(arguments.command, reporter)).into()
}
