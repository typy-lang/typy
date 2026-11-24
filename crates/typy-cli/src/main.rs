//! The `typy` command: `check`, `build` and `run` (CLI-01 to CLI-06).
#![forbid(unsafe_code)]

mod arguments;
mod cache;
mod command;
mod exit_status;
mod guard;
mod output;

use std::io::IsTerminal as _;

/// The stack the compiler runs on. Its passes recurse over the syntax tree, which nests up to
/// `typy_source::MAX_DEPTH` levels; the memory is reserved, and only what a program's nesting
/// uses is taken (NFR-09).
const COMPILER_STACK: usize = 512 * 1024 * 1024;

fn main() -> std::process::ExitCode {
    // `-C help` lists the code generation options and needs no file, as `rustc -C help` does.
    if asks_for_codegen_help(std::env::args().skip(1)) {
        for (key, meaning) in arguments::CODEGEN_KEYS {
            println!("-C {key:<22}{meaning}");
        }
        return std::process::ExitCode::SUCCESS;
    }
    let (arguments, matches) = arguments::Arguments::parse_with_matches(std::env::args_os())
        .unwrap_or_else(|error| error.exit());
    let coloured = output::colours(
        arguments.color,
        std::io::stderr().is_terminal(),
        std::env::var_os("NO_COLOR").as_deref(),
    );
    let reporter = output::Reporter::new(arguments.error_format, coloured);
    let global = arguments.global(&matches);
    let compiler = std::thread::Builder::new()
        .name("compiler".to_owned())
        .stack_size(COMPILER_STACK)
        .spawn(move || {
            guard::guarded(reporter, || {
                command::run(arguments.command, global, reporter)
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

/// Returns true when the command line, before any `--`, asks for `-C help` in any of its
/// spellings.
fn asks_for_codegen_help(arguments: impl Iterator<Item = String>) -> bool {
    let mut previous_is_codegen = false;
    for argument in arguments.take_while(|argument| argument != "--") {
        if (previous_is_codegen && argument == "help")
            || argument == "-Chelp"
            || argument == "-C=help"
        {
            return true;
        }
        previous_is_codegen = argument == "-C";
    }
    false
}

#[cfg(test)]
mod tests;
