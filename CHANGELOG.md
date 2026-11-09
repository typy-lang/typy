# Changelog

All notable changes to TyPy are recorded here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and TyPy uses
[semantic versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Tuples: displays such as `(1, "a")`, annotations such as
  `tuple[int, str]` and `tuple[()]`, items read at a literal position such
  as `pair[0]` or `pair[-1]`, `+`, `len`, comparisons item by item, truth
  tests, `str` and `repr`, and tuples as arguments and return values
  (DAT-08, TYP-06).
- Unpacking assignment into names and nested tuples of names, such as
  `a, b = b, a` and `x, (y, z) = t` (SMT-02).
- Lists of `int`, `float`, `bool`, `str` and lists: displays such as
  `[1, 2]`, annotations such as `list[str]`, an empty `[]` where an
  annotation, a return type or a parameter gives its type, items read and
  assigned at positions counted from either end with `IndexError` outside,
  `append`, `len`, `in` and `not in`, `==`, truth tests, `str` and `repr`,
  and `for` over a list. Two names bound to one list share it (DAT-07,
  DAT-11, TYP-05, EXP-07, CST-03). A list stores each item in eight bytes,
  so a `list[float]` holds raw doubles (MEM-02), and a million `int` items
  fit in 8.5 MB (NFR-13).
- Slices of strings and lists, such as `text[1:4]`, `xs[::-1]` and
  `xs[-2:]`, with Python's defaults and clamping, and `ValueError` for a
  zero step (DAT-12).
- The list methods `pop`, `insert`, `extend`, `remove`, `index`, `count`,
  `clear`, `copy` and `reverse`, with CPython's errors (LIB-03).
- `+` and `*` on lists, ordering of lists with `<`, `<=`, `>` and `>=`,
  `xs += ys` and `xs *= n` changing the list in place, and augmented
  assignment of a list item such as `counts[i] += 1` (EXP-02, EXP-06,
  SMT-01).
- An empty `[]` in a comparison takes the type of the other operand, so
  `xs == []` needs no annotation, and one inside a display takes the type
  of the other items, as in `[[1], []]` (TYP-05).
- `sorted`, with `reverse`, `list.sort`, `sum`, with an exact result for
  `int` items and CPython's compensated result for `float` items, `any`,
  `all`, and `min` and `max` of a list, with `default` (LIB-02, LIB-03).
  `sum` of a `list[float]` takes a start such as `0.0`, because CPython
  returns the `int` 0 for an empty list without one. The `key` argument
  arrives with function values in 1.0 (FUN-05).
- Lists of tuples and of `None`, such as `list[tuple[int, str]]`, with every
  list operation. A tuple is stored inline in the list, so a million pairs
  of `int` values take 16 MB (DAT-07, MEM-02).
- `list()`, `list(xs)`, and `list` of a `str`, `enumerate`, `zip` or
  `reversed`; `sorted`, `min`, `max`, `sum`, `any` and `all` take those
  sources too (LIB-02).
- `in` on a `str`, a substring test, and on a tuple, and slices of a tuple
  with literal bounds such as `pair[1:]` (EXP-07, DAT-12).
- `tuple[T, ...]`, a tuple of any length, with `tuple(source)`, indexing,
  slicing, `in`, `len`, `+`, `*`, comparisons, `count`, `index` and `for`.
  A tuple such as `(1, 2)` is accepted where a `tuple[int, ...]` is
  expected, and `for` goes over a tuple whose items have one type (DAT-14,
  CST-03).
- The `str` methods `find`, `rfind`, `index`, `rindex`, `count`,
  `startswith`, `endswith`, `split`, `rsplit`, `splitlines`, `partition`,
  `rpartition`, `join`, `strip`, `lstrip`, `rstrip`, `removeprefix`,
  `removesuffix`, `replace`, `center`, `ljust`, `rjust`, `zfill`,
  `expandtabs` and `isascii`, with arguments by keyword where CPython allows
  them (LIB-03).
- `round`, with and without `ndigits`, rounding half to even on the exact
  value as CPython does, so `round(2.675, 2)` is `2.67` (LIB-02).
- `for` over the characters of a `str`, and over `enumerate`, with
  `start`, `zip`, with `strict`, and `reversed` of lists and strings,
  nested as in `enumerate(zip(a, b))`, with CPython's errors (CST-03,
  LIB-02).

### Changed

- The runtime's size budget (NFR-02) is measured on the runtime a hello
  world links, 10.3 KB, rather than on the whole runtime archive, so the
  runtime can hold the built-in collections without enlarging programs
  that do not use them (ADR-0009).

### Fixed

- A program function named `range` is called by `for i in range(...)`,
  as in CPython, instead of the builtin (CST-02).
- A traceback shows an entry repeated in a row at most three times, then
  `[Previous line repeated N more times]`, as CPython does, so a
  `RecursionError` writes a few lines instead of one for each frame
  (STD-05).

## [0.1.0] - 2026-11-07

The first release. TyPy 0.1 compiles programs made of module-level
functions and statements over `int`, `float`, `str`, `bool` and `None` to
native executables for x86_64 Linux, with glibc or statically with musl.
Each program it accepts prints what CPython 3.12 to 3.15 prints. The
compiler needs glibc 2.39 or later and a C toolchain.

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
- The `typy-conformance` crate and a conformance suite of 89 programs in
  `tests/conformance/`: each accepted program runs under CPython 3.12 to
  3.15 and under TyPy in debug and release mode, with the same output,
  exit status and last line of standard error; each rejected program gets
  exactly its diagnostic code at its line and column. `make test` runs it.
- Every accepted conformance program also runs linked with AddressSanitizer
  and LeakSanitizer and must show no report. A program that ends with an
  uncaught exception frees the exception before it exits, so it leaks
  nothing.
- The sanitizer run uses a runtime built with AddressSanitizer, so every
  memory access the runtime makes is checked. `make miri` runs the
  runtime's unit tests under Miri, and `make fuzz` fuzzes the source,
  checking and lowering stages.
- Continuous integration: the quality gate and the release archive on
  every pull request, Miri and 30 minutes of fuzzing on demand before a
  release, code scanning, dependency review, and monthly grouped Dependabot
  updates. A tag `vX.Y.Z` releases the
  version after a maintainer approves it.
- `make dist` builds a reproducible release archive of the compiler and its
  runtime archives, and runs a program with it unpacked.
- `make check` measures branch coverage as well as line coverage and fails
  below 95 percent of either (NFR-11); `make coverage-branches` runs that
  measurement alone, and `make test-int`, `make test-e2e` and
  `make test-all` run one test level each.
- Golden tests of every text stage of `--emit` and of rendered
  diagnostics in `tests/golden/`; `make bless` rewrites them.
- End-to-end tests for a missing or failing C toolchain, an empty program,
  the 16 MiB limit, a file that is not UTF-8, two builds at once, argument
  forwarding by `typy run`, and environment variables that must not change
  the output.
- The stack check reads the stack pointer register, so `RecursionError`
  is raised in time also when locals do not live on the machine stack.
- The runtime archive is built for musl as well as glibc, and `make check`
  checks its size: 40.3 KB, and 29.8 KB without large integers (NFR-02). A
  hello world built with `--release-small` and linked statically with musl
  is 75 KB (NFR-01).
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

[Unreleased]: https://github.com/typy-lang/typy/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/typy-lang/typy/releases/tag/v0.1.0
