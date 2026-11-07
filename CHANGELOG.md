# Changelog

All notable changes to TyPy are recorded here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and TyPy uses
[semantic versioning](https://semver.org/spec/v2.0.0.html).

## Unreleased

### Added

- Engineering documents: research, requirements specification, use case
  and domain models, architecture, decision records ADR-0001 to ADR-0008,
  detailed design for 0.1, test plan and development guide.
- A development container with Rust 1.99.1, LLVM 23, musl, CPython 3.12 to
  3.15 for conformance testing, and the documentation tools.
- The `typy-diagnostics` crate: source spans, diagnostic codes, the code
  registry and text rendering, with unit and property tests.
- The `typy-source` crate: reading source files with the 16 MiB and UTF-8
  checks, line and column positions, parsing with the program's Python
  floor, and TyPy's own syntax tree with its text dump.
- The `typy-check` crate: the 0.1 feature table, name resolution with
  Python's scoping, type inference within a function, definite assignment,
  and the typing rules for the operators and builtins of 0.1. Every
  rejection names its rule.
- The `typy-tyir` crate: TyIR's types, a function builder, a verifier that
  checks definitions by dominance and the types of every operation, and the
  text form for `--emit=tyir`.
- The `typy-lower` crate: lowers a checked program to TyIR with reference
  counting, error returns through `invoke`, and the semantics of CPython for
  loops, `and`, `or`, comparison chains and the builtins of 0.1.
- The `typy-codegen` crate: turns TyIR into LLVM 23 IR, assembly and object
  code, with static immortal constants and traceback entries on raising
  calls; `--release-small` marks functions `minsize`, as LLVM 23 has no `Oz`.
- The `typy-runtime` crate, without the standard library: object headers,
  reference counting, `str` objects and their operations, exceptions with
  tracebacks and the uncaught-exception report, buffered output, the stack
  check and `main`. `make runtime` builds the archive programs link.
- Unlimited-size `int` in the runtime: arithmetic, `//`, `%` and `**` with
  Python's floor rules, bitwise operators and shifts, comparisons, and
  conversion to and from `str` with `int()`'s parsing rules and CPython's
  4300-digit limit. Error messages match CPython's.
- `float` in the runtime: `/`, `//`, `%`, `**` and `abs` with CPython's
  special cases; `int / int`, `float(int)` and `float(str)` rounded once to
  the nearest double; exact `int(float)` and exact comparison of `int` with
  `float`; and `repr` with CPython's shortest digits and layout.
- Every division by zero raises `ZeroDivisionError: division by zero`, the
  wording of CPython 3.14 and later (OI-14).
- `repr` of a string escapes exactly the code points CPython 3.15 escapes,
  from Unicode 17.0 tables generated from CPython's own data (OI-15).
- `chr` and `ord` in the runtime, with CPython's messages.
- `int()` and `float()` read decimal digits of every script, such as
  `int("\u0661\u0662")`, which is `12`.
- The `typy-build` crate: build requests, modes and `--emit` stages, and
  the link of an object file with the runtime archive through `cc` or
  `musl-gcc`. The executable appears whole or not at all; a missing
  toolchain program is E0701, a failed link E0702, and a missing runtime
  archive, or one from another TyPy version, E0703.
- Diagnostics without a source location, such as a failed link.
- The `typy` command: `typy check FILE`, `typy build FILE` with `--emit`,
  `-o`, `--release` and `--release-small`, and `typy run FILE -- ARGS`,
  which keeps each build in a cache under `TYPY_HOME` and runs it again
  without compiling. Exit status 0 on success, 1 for program errors, 2 for
  wrong usage and 101 for an internal error, which a panic also becomes.
- E0106 when an output file cannot be written; the linker reports a
  directory it cannot write with E0106 instead of E0702.
- The `typy-driver` crate: runs loading, checking, lowering, code
  generation and linking in order, and stops after the stage `--emit`
  names. `--emit=typed-ast` writes every node with its position and type,
  and `--emit=tyir` leaves out the `inc` and `dec` that `--emit=tyir-mem`
  shows.
- A quality gate, `make check`, that checks formatting, runs Clippy and the
  tests with at least 95 percent line coverage, lints the documents, checks for
  non-ASCII text, checks that every requirement has a row in the test plan,
  and checks that every diagram is rendered from its source.
