# ADR-0104: Spell common options as flags and the rest as `-C KEY=VALUE`

## Status

Accepted

## Date

2026-11-23

## Context

`typy` offers two optimisation modes beside the debug build, no choice of
processor, no timings, no shown commands, no kept intermediate files and no
per-code warning levels: the research note "the command lines of
compilers" found these gaps against `rustc`, `clang`, `cargo` and CPython.
Its experiment X10 measured one more choice: linking the runtime into a
program as LLVM bitcode makes it 6 percent faster at twenty times the build
time. Each new option needs a spelling, and the spelling decides how
`typy --help` reads as options grow.

## Options considered

1. **A flag for each option,** as `clang` and CPython do. Each is easy to
   find, but `--help` grows with every code generation detail.
2. **Few flags, and nearly everything through `-C KEY=VALUE` and
   `-Z KEY=VALUE`,** as `rustc` does. It scales, but a Python user meets
   unfamiliar spellings for the common choices.
3. **Flags for the common options, spelt as `clang` and CPython spell
   them, and `-C KEY=VALUE` for the rest of code generation.**

## Decision

Option 3, chosen by the project on 2026-11-23. The common options are
flags: `-O0` to `-O3`, `-Os` and `-Oz`, `--target-cpu`, `-g`, `-v`,
`--timings`, `--save-temps`, `--color`, `-W`, `-A` and `-D`, and `typy run
-c` and `-m`. `-C KEY=VALUE` takes the rest of code generation, such as
`opt-level`, `target-cpu`, `llvm-args`, `debuginfo`, `inline-runtime` and
`remark`, and a flag and its `-C` key are the same setting. Choices that
trade a promise for speed are opt-in and off by default:
`--target-cpu native`, which makes a build depend on the machine that made
it against NFR-06, and `-C inline-runtime=yes`, which costs build time.
Every option that changes a build's output joins the key of `typy run`'s
cache.

## Consequences

- `--help` lists the options most users need, and `-C help` lists the
  rest, as `rustc -C help` does.
- `--release` stays `-O2` and `--release-small` stays `-Oz`, so existing
  commands keep their meaning.
- A default build stays portable and reproducible, and fast to build.

## Related requirements

CLI-05, CLI-11, CLI-20 to CLI-31, CLI-33 to CLI-37, NFR-05, NFR-06.
