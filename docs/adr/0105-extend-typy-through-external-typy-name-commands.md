# ADR-0105: Extend `typy` through external `typy-NAME` commands

## Status

Accepted

## Date

2026-11-23

## Context

A project needs more than a compiler: a test runner, benchmarks, a
formatter, a way to clear the build cache. `cargo` gives each a command,
`cargo test`, `cargo fmt`, and finds those it does not hold as `cargo-NAME`
programs on `PATH`; `git` finds `git-NAME` the same way. The research note
"the command lines of compilers" asked whether `typy` gains such commands.

## Options considered

1. **Commands inside `typy`,** as `cargo build` and `cargo test` are.
   Every tool then shares the compiler's release, and the compiler, which
   every build trusts, grows with tools that have nothing to do with
   compiling.
2. **External commands:** `typy NAME ARGS`, for a NAME that is not one of
   `typy`'s own, runs the program `typy-NAME` found on `PATH`, as `git` and
   `cargo` do. Each tool is released, replaced and written on its own, by
   the project or by anyone.
3. **No commands beyond the compiler's;** each tool is run by its own name.

## Decision

Option 2, chosen by the project on 2026-11-23. `typy` stays the compiler:
`check`, `build`, `run`, `explain` and `print`. For any other NAME it runs
`typy-NAME` from `PATH` with the remaining arguments, its standard streams
and its exit status passed through, and with `TYPY` set to the path of the
running `typy`, so the tool uses the same compiler. A NAME with no such
program is an unknown command, exit status 2 (CLI-06). The interface a tool
relies on is the compiler's: `--error-format json`, `typy print` for paths
and settings, and the exit statuses of CLI-06.

## Consequences

- The compiler keeps a small surface that changes carefully; tools change
  at their own pace.
- Running a program from `PATH` trusts `PATH`, as `git` and `cargo` do; the
  documentation says so, and `typy` never runs one for its own commands.
- `TYPY` is set for the child and never read by `typy`, so CLI-11 holds.

## Related requirements

CLI-06, CLI-11, CLI-25, CLI-32.
