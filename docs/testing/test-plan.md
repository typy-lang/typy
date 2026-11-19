# TyPy test plan

| Field | Value |
| --- | --- |
| Document | TyPy test plan |
| Version | 1.07 |
| Status | Draft |
| Owner | typy-lang contributors |
| Last updated | 2026-11-19 |

## 1. Purpose

This document says how TyPy is tested: the test levels, how tests are
designed at each level, where they run, when a release may ship, and which
tests cover each requirement. It builds on the approved
[detailed design](../design/detailed-design.md), the
[software requirements specification](../requirements/srs.md), version
1.2, and the decision records ADR-0001 to ADR-0008.

The specification defines correct behaviour, and CPython is the oracle for
it (ADR-0008). A test therefore states which requirement it covers, and an
accepted-program test needs no hand-written expected output: CPython
provides it.

## 2. Test levels

Each test sits at the lowest level that can prove its behaviour.

| Level | What it proves | Where | Runs |
| --- | --- | --- | --- |
| Unit (UT) | One function or type in isolation: parsing a code, normalising a union, a large-integer operation | `#[cfg(test)]` modules in each crate | Every commit |
| Golden | The text of one compiler stage for one input, through `--emit` (CLI-04) | `tests/golden/`, run by `crates/typy-driver/tests/integration/golden.rs` | Every commit |
| Integration (IT) | The compiler, runtime and C toolchain together on one program, compared with CPython; the conformance suite | `tests/conformance/<area>/` through `typy-conformance`, which drives `typy-driver` in process | Every commit |
| End-to-end (E2E) | The `typy` executable as a user runs it: arguments, exit status, files written, and the built program run as a subprocess | `crates/typy-cli/tests/end_to_end/cli.rs` | Every commit |
| Fuzz | No input crashes the compiler (NFR-09) | `fuzz/` | 30 minutes before each release; 60 seconds on demand |
| Benchmark | The numbered budgets of NFR-03 to NFR-05, NFR-13 and NFR-14, and how TyPy compares with other languages | `tests/bench/` | `make bench` in CI from 0.3 for NFR-04, NFR-05 and NFR-14; the run-time suite of NFR-03 before a release, from 1.0; `make bench-compare`, which writes the [benchmark report](benchmark-report.md), on demand from 0.5 |
| Mutation | Tests fail when the code they cover is changed | `make mutants` over the lines a change adds or alters, each mutant against its own crate's tests, in a container limited to 6 GB with no swap, since a mutant can turn a bounded loop into one that allocates until memory runs out. `typy-build`, `typy-driver` and `typy-cli` are left out, since their tests need the runtime archives built beside the compiler; the end-to-end and conformance tests cover them. A whole crate is run with `make mutants-package` when its tests are reviewed | On demand, for a change whose tests deserve a closer look |

## 3. Test design

### 3.1 Unit tests

- Each public function has tests for its normal cases, its boundaries and
  each error it returns. Variations of one behaviour are a table of cases
  in one test.
- Parsers and normalisers have property-based tests with `proptest`:
  `DiagnosticCode::parse` and `Display` round-trip; `LineIndex` agrees with
  a reference that counts code points; union normalisation is idempotent and
  ignores order.
- The runtime's large-integer operations are checked against the
  `num-bigint` crate, as a test-only dependency, on generated values that
  include the inline boundaries -2^62, -2^62 - 1, 2^62 - 1 and 2^62.
- `float_repr` is checked by property: the text it writes parses back to the
  same double, and its digits are the shortest that do. A fixed table of
  values, including `0.1`, `1e16`, `1e-05`, `-0.0`, `inf` and `nan`, holds
  the text CPython writes for each.
- A test name is a sentence about behaviour, such as
  `oversized_source_is_refused_before_reading`.

### 3.2 Golden tests

A golden test is a `.py` input and the expected output of one stage. The
programs are in `tests/golden/programs/`, and the text of each stage but
`obj` is in `tests/golden/<stage>/<program>.txt`. The programs in
`tests/golden/diagnostics/`, each named for the DIA requirement it shows,
sit beside the rendered text of their diagnostics. The expected files
change only through `make bless`, and the diff is reviewed in the same
commit. A test never edits its expected file to pass, and an expected file
without its program fails the run. The golden programs are ASCII, so the
files follow the repository's ASCII rule; Unicode text is tested by the
conformance suite.

### 3.3 Conformance tests

Conformance tests are the main integration level. Each is a Python program
in the format of design section 15: comments give the requirement IDs, the
expectation, and an optional Python floor.

| Kind | Expectation | Passes when |
| --- | --- | --- |
| Accepted | `accepted` | Every oracle version from the floor agrees, and the debug and release builds run alike with them: standard output, exit status and last line of standard error (STD-02, NFR-15) |
| Rejected | `rejected CODE at LINE:COLUMN` | `typy check` reports exactly that code at that position, and nothing else |

Every requirement of areas STD, LEX, TYP, DAT, EXP, SMT, CST, FUN, CLS,
GEN, EXC, MOD, LIB, MEM and FFI has at least one accepted and one rejected
test (NFR-10). Each test is designed with:

- **Equivalence classes and boundaries.** For each operator and type pair,
  one value from each class and every boundary: zero, the inline integer
  limits, negative values, empty strings, the first and last index, and one
  past each.
- **Error paths.** Each exception the runtime raises has a test that lets
  it go uncaught and, from 0.3, one that catches it.
- **The excluded features.** Each item of specification section 6.1 has a
  rejected test.
- **Oracle disagreement.** A test whose oracle versions disagree is reported
  as invalid, not failed (STD-03). The test is then rewritten so the
  versions agree, or kept as a documented record of the difference.

A test file is named after its first requirement and its behaviour, such as
`EXP-03_float_division_by_zero_raises.py`. A test of several modules is a
directory named so, whose `main.py` holds the header and whose other files
are its modules, such as `mod/MOD-02_packages_and_relative_imports/`.

Fourteen requirements have no rejected file, which `NO_REJECTED_FILE` in
`suite.rs` lists. LEX-01 has none because a source that is not UTF-8 would
break the repository's ASCII rule, and LEX-09 because a source over 16 MiB
is too large to commit; unit tests in `typy-source` build those inputs when
they run and check E0102 and E0101. The others have no rule a program can
break: memory management (MEM-01, MEM-02, MEM-03, MEM-04, MEM-09),
recursion (FUN-02), a surrogate escape (LEX-10), the order of evaluation
and of module initialisation (EXP-01, MOD-03), the report of an uncaught
exception (STD-05) and the class and text of an error the C library
reports (EXC-19) describe what an accepted program does, and the warning
of CST-07 accepts the program it warns about. Where a 0.1 program
cannot show the rejection a requirement describes, its rejected test records
what 0.1 does instead, such as a function used as a truth value, E0201
until functions are values in 1.0.

The test `crates/typy-conformance/tests/integration/suite.rs` runs the
suite on every commit and fails when a test does not pass or a 0.1
requirement of the matrix lacks either kind of test (NFR-10). The 0.1
corpus holds 45 accepted and 44 rejected tests and runs in about 20
seconds.

### 3.4 End-to-end tests

E2E tests run the `typy` executable as a subprocess, in a temporary
directory. They cover each command and flag, each exit status of CLI-06,
and the failure of each dependency:

| Condition | Expected |
| --- | --- |
| Missing source file | Exit status 1, with the path |
| Empty source file | Accepted; the program prints nothing and exits 0 |
| Source file of exactly 16 MiB, and of 16 MiB and one byte | Accepted; refused with E0101 |
| Source that is not UTF-8 | E0102 at the first invalid byte |
| Unknown flag or command | Exit status 2 |
| C toolchain missing from `PATH` | E0701, and no output file |
| Linker fails | E0702, and no output file |
| Output directory not writable | Exit status 1, with the path |
| Two builds of the same program at once into different outputs | Both succeed with identical bytes |

Subprocess runs are included in coverage (NFR-11).

### 3.5 Memory and safety

The test `crates/typy-conformance/tests/integration/sanitized.rs` builds every accepted
conformance test once more: its object file is linked with the runtime built
with AddressSanitizer (`-Zsanitizer=address`, pinned nightly) and with GCC's
AddressSanitizer library, and run with leak detection on. The run must show
CPython's results; a sanitizer report ends it with status 86 and fails the
test (NFR-07, NFR-08). Every allocation and free is checked, leaks are
reported at exit, and every memory access the runtime makes is checked.
Generated code is not instrumented; it is checked by the TyIR verifier, and
instrumenting it is planned for 0.2 (OI-16).

`make miri` runs the runtime's unit tests under Miri (ADR-0006), which
reports out-of-bounds, misaligned and dangling accesses and reads of
uninitialised memory. Miri cannot call `pow` or `getrlimit`, so the three
tests that do are skipped under it, as is the 4300-digit conversion, which
takes hours there; the long randomised loops check a sample. The unit tests
do not release the objects they make, so Miri's leak report is off; whole
programs are checked for leaks by the sanitizer run.

### 3.6 Fuzzing

The fuzz target `stages` (`fuzz/fuzz_targets/stages.rs`) feeds generated
input to the source, checking and lowering stages in turn, starting from the
conformance corpus. A crash, a hang over 10 seconds, or an internal error
E9001, which a TyIR verifier failure becomes, is a defect. Each one found
becomes a regression test named for its behaviour. `make fuzz` runs it for
`FUZZ_SECONDS` seconds, 60 by default, with libFuzzer and AddressSanitizer;
the 30-minute run of NFR-09 is `FUZZ_SECONDS=1800 make fuzz`. The front end
of 0.1 is small, and a 60-second run already stops finding new code paths
quickly, so a longer run adds little; the budget grows with the language.

### 3.7 Defects

A defect is reproduced by a failing test before the code is changed, and
the test stays, named for the behaviour it checks.

## 4. Environments

| Environment | Contents | Used by |
| --- | --- | --- |
| Dev container | `typy-lang-dev`: Rust 1.99.1, the cargo tools, LLVM 23.1.6 from the archive `.devcontainer/llvm.lock` pins, musl, Node and markdownlint | UT, golden, E2E, local runs of everything |
| Oracle interpreters | CPython 3.12.16, 3.13.18, 3.14.10 and 3.15.2, exact builds installed with uv 0.13.7 in the dev container, as `python3.12` to `python3.15`, with nothing else installed | Conformance |
| Nightly Rust | One pinned nightly toolchain, for Miri and the sanitizers only | Section 3.5 |
| Diagrams container | PlantUML 1.2026.8, pinned by digest | Documentation gate |
| Documentation tools | Zensical 0.1.1 and its dependencies, pinned with their sha256 in `scripts/docs-requirements.txt`, in a CPython 3.14 environment at `/var/cache/typy/docs` in the dev container, and on the runner of the Pages workflow | Documentation gate, DOC-05 |

Each release adds its platforms to the CI matrix as section 2.3 of the
specification lists them: Linux x86_64 in 0.1, Linux AArch64 in 0.6, macOS
AArch64 in 0.7, Windows x86_64 in 1.0.

From 0.7 every host builds with the LLVM 23.1.6 that TyPy builds from its
sources and pins by its SHA-256 (ADR-0054), and a `macos-15` CI job, on
GitHub's free arm64 macOS runner, runs the unit, integration, end-to-end
and conformance tests natively on macOS against CPython for macOS
(NFR-17); macOS work does not use the dev container (ADR-0055).

From 0.6 the dev container also holds Debian's AArch64 cross toolchain
(`gcc-aarch64-linux-gnu`, `libc6-dev-arm64-cross`), `qemu-user` and Rust's
`aarch64-unknown-linux-gnu` target. A local run builds the conformance
suite for AArch64 and runs each program under `qemu-aarch64`; CI runs the
unit, integration and conformance tests natively on an `ubuntu-24.04-arm`
runner, which is free for a public repository. The arm64 job builds the
dev image for arm64 and leaves out the coverage build, which the runner's
14 GB disk cannot hold beside the image.

## 5. Entry and exit criteria

### 5.1 Entry

| Level | A change enters testing when |
| --- | --- |
| UT, golden | It builds, and `cargo fmt --check` and Clippy pass |
| IT | Its UT and golden tests pass |
| E2E | Its IT tests pass |
| Release candidate | Every level passes on the release's whole platform matrix |

### 5.2 Exit, for a release

A release ships when all of these hold:

1. Every M requirement of the release, and every earlier one, has its
   accepted and rejected tests, and they pass on every platform of the
   release.
2. Every S or C requirement of the release passes, or the specification
   moves it to a later release, saying so.
3. Line and branch coverage is at least 95 percent (NFR-11).
4. The sanitizer and Miri runs report nothing (NFR-07, NFR-08).
5. Fuzzing has run 30 minutes with no new crash (NFR-09).
6. Every numbered non-functional requirement whose release has been reached
   is within its budget.
7. No defect remains open that breaks a requirement of the release.

## 6. Test data

- Test programs are committed as Python source. No binary file is
  committed.
- Large inputs, such as the 16 MiB source file and the 10,000-line
  project, are generated when the tests start, from a seed committed with
  the test.
- The benchmark programs and their C versions are committed as source. The
  build-time projects are written by `tests/bench/build_time.py` when it
  starts: a 10,000-line project in one file and in 200 modules, and 50 and
  200 modules whose check times it compares. Each figure is the best of three
  runs, and memory is the peak resident size of each build.

## 7. Risks

| Risk | Effect | Mitigation |
| --- | --- | --- |
| The oracle versions disagree, through Unicode databases or exception messages | Tests become invalid | Reported as invalid, not failed; each one reviewed and documented |
| Running four oracle versions and two build modes per test makes CI slow | Slow feedback | Oracle results are cached by the hash of the test file and the oracle image digest |
| Benchmarks vary with machine load | False failures | Budgets are checked on a dedicated runner, as the median of five runs |
| Miri and the sanitizers need a nightly toolchain | Nightly breakage | One nightly date is pinned and moved on purpose |
| Fuzzing finds many defects early | Release delay | Each defect gets a test first; fuzz findings are triaged before each release |

## 8. Traceability matrix

Each requirement, the release that first meets it, its test level, and
where its tests live. Constraints that a test can check are included.
`scripts/check-traceability.sh` fails the quality gate when a requirement
of the specification is missing here, or when any document cites an ID
that the specification does not define.

| Requirement | Release | Level | Tests |
| --- | --- | --- | --- |
| STD-01 | 0.1 | IT | `tests/conformance/std/STD-01_*.py` |
| STD-02 | 0.1 | IT | `tests/conformance/std/STD-02_*.py` |
| STD-03 | 0.1 | IT | `tests/conformance/std/STD-03_*.py` |
| STD-04 | 0.1 | IT | `tests/conformance/std/STD-04_*.py` |
| STD-05 | 0.3 | IT | `tests/conformance/std/STD-05_*.py` |
| STD-06 | 0.1 | IT | `tests/conformance/std/STD-06_*.py` |
| STD-07 | 0.1 | IT | `tests/conformance/std/STD-07_*.py` |
| CLI-01 | 0.1 | E2E | `crates/typy-cli/tests/end_to_end/cli.rs`, tests named for CLI-01 |
| CLI-02 | 0.1 | E2E | `crates/typy-cli/tests/end_to_end/cli.rs`, tests named for CLI-02 |
| CLI-03 | 0.1 | E2E | `crates/typy-cli/tests/end_to_end/cli.rs`, tests named for CLI-03 |
| CLI-04 | 0.1 | E2E, golden | `crates/typy-cli/tests/end_to_end/cli.rs`, tests named for CLI-04; stage golden files in `tests/golden/` |
| CLI-05 | 0.1 | E2E | `crates/typy-cli/tests/end_to_end/cli.rs`, tests named for CLI-05 |
| CLI-06 | 0.1 | E2E | `crates/typy-cli/tests/end_to_end/cli.rs`, tests named for CLI-06 |
| CLI-07 | 0.3 | E2E | `crates/typy-cli/tests/end_to_end/cli.rs`, tests named for CLI-07 |
| CLI-08 | 0.6 | UT, E2E | `crates/typy-codegen/src/target.rs` for the supported triples, the host and the triples each host builds for; `crates/typy-cli/tests/end_to_end/cli.rs` for `--target` on `build` and `check`, an unsupported triple, another system's triple and an AArch64 executable built on an x86_64 host and run under `qemu-aarch64` |
| CLI-14 | 0.6 | UT, E2E | `crates/typy-build/src/toolchain.rs` for the tools of each target on each host; `crates/typy-build/src/linker.rs` and `library_link.rs` for the tools they run; `crates/typy-source/src/settings_file/target.rs` for the `typy.toml` table of a target; `crates/typy-build/src/toolchain.rs` for the triples it shares with the code generator; `crates/typy-cli/tests/end_to_end/cli.rs` for E0701 naming a missing cross tool |
| CLI-15 | 0.7 | UT, E2E | `crates/typy-codegen/src/target.rs` for `aarch64-apple-darwin`, its LLVM triple and CPU; `crates/typy-codegen/tests/codegen.rs` for a deterministic Mach-O object of every test program; the `macos-15` CI job runs every unit, integration and end-to-end test natively on macOS, and builds and runs a program from the macOS release archive |
| CLI-16 | 0.7 | UT, E2E | `crates/typy-build/src/library_layout.rs` for the macOS names, install name and versions; `crates/typy-build/src/mach_o.rs` for the arguments of Apple's linker and the exports list; `crates/typy-cli/tests/end_to_end/library.rs` on the `macos-15` job, linking a C host with each library |
| CLI-09 | 0.5 | UT, E2E | `crates/typy-build/src/request.rs` and `library_kind.rs` for the output's name; `crates/typy-cli/tests/end_to_end/library.rs`, tests named for CLI-09: a static library in a C program, its only global symbols, a shared library from CPython through `ctypes`, two libraries in one program with a call from another thread, and a file name that is not a module name |
| CLI-13 | 0.5 | UT, E2E | `crates/typy-source/src/version.rs` for the release of a version; `crates/typy-source/src/settings_file.rs` for `version`; `crates/typy-check/tests/integration/exports.rs` for `__version__` and E0429; `crates/typy-build/src/library_layout.rs` for each layout; `crates/typy-cli/tests/end_to_end/library.rs`, tests named for CLI-13: a shared library's soname and links, a new release that moves them, a static library's link, `-o`, and two versions that differ |
| CLI-10 | 0.3 | E2E | `crates/typy-cli/tests/end_to_end/cli.rs`, tests named for CLI-10 |
| CLI-11 | 0.1 | E2E | `crates/typy-cli/tests/end_to_end/cli.rs`, tests named for CLI-11 |
| CLI-12 | 0.3 | E2E | `crates/typy-cli/tests/end_to_end/cli.rs`, tests named for CLI-12 |
| LEX-01 | 0.1 | IT | `tests/conformance/lex/LEX-01_*.py` |
| LEX-02 | 0.1 | IT | `tests/conformance/lex/LEX-02_*.py` |
| LEX-03 | 0.1 | IT | `tests/conformance/lex/LEX-03_*.py` |
| LEX-04 | 0.1 | IT | `tests/conformance/lex/LEX-04_*.py` |
| LEX-05 | 0.1 | IT | `tests/conformance/lex/LEX-05_*.py` |
| LEX-06 | 0.2 | IT | `tests/conformance/lex/LEX-06_*.py` |
| LEX-07 | 0.2 | IT | `tests/conformance/lex/LEX-07_*.py` |
| LEX-08 | 0.1 | IT | `tests/conformance/lex/LEX-08_*.py` |
| LEX-09 | 0.1 | IT | `tests/conformance/lex/LEX-09_*.py` |
| LEX-10 | 0.5 | IT | `tests/conformance/lex/LEX-10_*.py` |
| LEX-11 | 0.3 | UT, IT, E2E | `crates/typy-source/src/parse/tests.rs` for each limit just inside and just past it; `crates/typy-tyir/src/verify/tests.rs` for a function of 100000 blocks; `crates/typy-cli/tests/end_to_end/nesting.rs` for a program as deep as CPython compiles and each refusal; `tests/conformance/lex/LEX-11_*.py` |
| TYP-01 | 0.1 | IT | `tests/conformance/typ/TYP-01_*.py` |
| TYP-02 | 0.1 | IT | `tests/conformance/typ/TYP-02_*.py` |
| TYP-03 | 0.1 | IT | `tests/conformance/typ/TYP-03_*.py` |
| TYP-04 | 0.1 | IT | `tests/conformance/typ/TYP-04_*.py` |
| TYP-05 | 0.2 | IT | `tests/conformance/typ/TYP-05_*.py` |
| TYP-06 | 0.2 | IT | `tests/conformance/typ/TYP-06_*.py` |
| TYP-07 | 0.4 | IT | `tests/conformance/typ/TYP-07_*.py` |
| TYP-08 | 0.4 | IT | `tests/conformance/typ/TYP-08_*.py` |
| TYP-09 | 0.1 | IT | `tests/conformance/typ/TYP-09_*.py` |
| TYP-10 | 0.1 | IT | `tests/conformance/typ/TYP-10_*.py` |
| TYP-11 | 0.1 | IT | `tests/conformance/typ/TYP-11_*.py` |
| TYP-12 | 0.1 | IT | `tests/conformance/typ/TYP-12_*.py` |
| TYP-13 | 0.2 | IT | `tests/conformance/typ/TYP-13_*.py` |
| TYP-14 | 0.4 | IT | `tests/conformance/typ/TYP-14_*.py` |
| TYP-15 | 0.3 | IT | `tests/conformance/typ/TYP-15_*.py` |
| TYP-16 | 0.5 | IT | `tests/conformance/typ/TYP-16_*.py` |
| TYP-17 | 0.5 | IT | `tests/conformance/typ/TYP-17_*.py` |
| TYP-18 | 0.10 | IT | `tests/conformance/typ/TYP-18_*.py` |
| TYP-19 | 1.0 | IT | `tests/conformance/typ/TYP-19_*.py` |
| TYP-20 | 0.9 | UT, IT | `typy-check` tests of the classes that fit `Iterator[T]` and `Iterable[T]`, and of those that do not; `tests/conformance/typ/TYP-20_*.py`, one function given each kind of iterator, also in the sanitizer run |
| TYP-21 | 0.9 | IT | `tests/conformance/typ/TYP-21_*.py` |
| DAT-01 | 0.1 | IT | `tests/conformance/dat/DAT-01_*.py` |
| DAT-02 | 0.1 | IT | `tests/conformance/dat/DAT-02_*.py` |
| DAT-03 | 0.1 | IT | `tests/conformance/dat/DAT-03_*.py` |
| DAT-04 | 0.1 | IT | `tests/conformance/dat/DAT-04_*.py` |
| DAT-05 | 0.1 | IT | `tests/conformance/dat/DAT-05_*.py` |
| DAT-06 | 0.2 | IT | `tests/conformance/dat/DAT-06_*.py` |
| DAT-07 | 0.2 | IT | `tests/conformance/dat/DAT-07_*.py` |
| DAT-08 | 0.2 | IT | `tests/conformance/dat/DAT-08_*.py` |
| DAT-09 | 0.2 | IT | `tests/conformance/dat/DAT-09_*.py` |
| DAT-10 | 0.2 | IT | `tests/conformance/dat/DAT-10_*.py` |
| DAT-11 | 0.2 | IT | `tests/conformance/dat/DAT-11_*.py` |
| DAT-12 | 0.2 | IT | `tests/conformance/dat/DAT-12_*.py` |
| DAT-13 | 0.3 | IT | `tests/conformance/dat/DAT-13_*.py` |
| DAT-14 | 0.2 | IT | `tests/conformance/dat/DAT-14_*.py` |
| DAT-15 | 0.2 | IT | `tests/conformance/dat/DAT-15_*.py` |
| DAT-16 | 0.3 | IT | `tests/conformance/dat/DAT-16_*.py` |
| DAT-17 | 0.3 | IT | `tests/conformance/dat/DAT-17_*.py` |
| EXP-01 | 0.1 | IT | `tests/conformance/exp/EXP-01_*.py` |
| EXP-02 | 0.1 | IT | `tests/conformance/exp/EXP-02_*.py` |
| EXP-03 | 0.1 | IT | `tests/conformance/exp/EXP-03_*.py` |
| EXP-04 | 0.1 | IT | `tests/conformance/exp/EXP-04_*.py` |
| EXP-05 | 0.1 | IT | `tests/conformance/exp/EXP-05_*.py` |
| EXP-06 | 0.1 | IT | `tests/conformance/exp/EXP-06_*.py` |
| EXP-07 | 0.2 | IT | `tests/conformance/exp/EXP-07_*.py` |
| EXP-08 | 0.1 | IT | `tests/conformance/exp/EXP-08_*.py` |
| EXP-09 | 0.1 | IT | `tests/conformance/exp/EXP-09_*.py` |
| EXP-10 | 0.1 | IT | `tests/conformance/exp/EXP-10_*.py` |
| EXP-11 | 0.1 | IT | `tests/conformance/exp/EXP-11_*.py` |
| EXP-12 | 0.2 | IT | `tests/conformance/exp/EXP-12_*.py` |
| EXP-13 | 0.5 | IT | `tests/conformance/exp/EXP-13_*.py` |
| EXP-14 | 0.8 | UT, IT | `typy-check` tests of what a `lambda` captures, a comprehension's variable and the variable it is assigned to included, of each context that gives its types and of the refusals; `tests/conformance/exp/EXP-14_*.py`, late binding in a loop and in a comprehension run twice included, also in the sanitizer run |
| EXP-15 | 0.9 | IT | `tests/conformance/exp/EXP-15_*.py`, the first iterable evaluated at once and the rest lazily, also in the sanitizer run |
| EXP-16 | 0.1 | IT | `tests/conformance/exp/EXP-16_*.py` |
| EXP-17 | 0.9 | UT, IT | `typy-check` tests of the element types of `*args` and `**kwargs` and of each unpacking; `tests/conformance/exp/EXP-17_*.py` |
| SMT-01 | 0.1 | IT | `tests/conformance/smt/SMT-01_*.py` |
| SMT-02 | 0.2 | IT | `tests/conformance/smt/SMT-02_*.py` |
| SMT-03 | 0.1 | IT | `tests/conformance/smt/SMT-03_*.py` |
| SMT-04 | 0.3 | IT | `tests/conformance/smt/SMT-04_*.py` |
| SMT-05 | 0.3 | IT | `tests/conformance/smt/SMT-05_*.py` |
| SMT-06 | 0.3 | IT | `tests/conformance/smt/SMT-06_*.py` |
| SMT-07 | 0.3 | IT | `tests/conformance/smt/SMT-07_*.py` |
| SMT-08 | 0.3 | IT | `tests/conformance/smt/SMT-08_*.py` |
| SMT-09 | 0.8 | UT, IT | `typy-check` tests of CPython's rules for `nonlocal`, of the cell it shares, through a function between included, and of the narrowing a call may undo; `tests/conformance/smt/SMT-09_*.py`, also in the sanitizer run |
| SMT-10 | 0.4 | IT | `tests/conformance/smt/SMT-10_*.py` |
| SMT-11 | 0.9 | UT, IT | `typy-check` tests of each statement form of `yield`, of the `S \| None` an assigned `yield` receives, and of the forms planned for 1.0; `typy-tyir` verifier tests that no value lives across a yield; `tests/conformance/smt/SMT-11_*.py`, also in the sanitizer run |
| SMT-12 | 0.5 | IT | `tests/conformance/smt/SMT-12_*.py` |
| CST-01 | 0.1 | IT | `tests/conformance/cst/CST-01_*.py` |
| CST-02 | 0.1 | IT | `tests/conformance/cst/CST-02_*.py` |
| CST-03 | 0.2 | IT | `tests/conformance/cst/CST-03_*.py` |
| CST-04 | 0.3 | IT | `tests/conformance/cst/CST-04_*.py` |
| CST-05 | 0.3 | IT | `tests/conformance/cst/CST-05_*.py` |
| CST-06 | 0.5 | IT | `tests/conformance/cst/CST-06_*.py` |
| CST-07 | 0.5 | IT | `tests/conformance/cst/CST-07_*.py` for complete matches; `crates/typy-check/tests/integration/matching.rs` for W0401 |
| CST-08 | 0.3 | IT | `tests/conformance/cst/CST-08_*.py` |
| CST-09 | 0.3 | IT | `tests/conformance/cst/CST-09_*.py` |
| CST-10 | 0.3 | IT | `tests/conformance/cst/CST-10_*.py` |
| CST-11 | 1.0 | IT | `tests/conformance/cst/CST-11_*.py` |
| FUN-01 | 0.1 | IT | `tests/conformance/fun/FUN-01_*.py` |
| FUN-02 | 0.1 | IT | `tests/conformance/fun/FUN-02_*.py` |
| FUN-03 | 0.8 | UT, IT | `typy-check` tests of binding arguments to keyword-only and positional-only parameters; `tests/conformance/fun/FUN-03_*.py` |
| FUN-04 | 0.8 | UT, IT | `typy-check` and `typy-lower` tests of each variable's capture, by value or in a cell, of recursion and of mutual recursion; `tests/conformance/fun/FUN-04_*.py`, also in the sanitizer run, which shows each closure, cell and shared environment freed (NFR-07, NFR-08) |
| FUN-05 | 0.8 | UT, IT | `typy-codegen` tests of a function object, its descriptor and a call through a value; `tests/conformance/fun/FUN-05_*.py` for identity, equality, hashing and `repr`, bound methods and `key`, also in the sanitizer run |
| FUN-06 | 0.10 | IT | `tests/conformance/fun/FUN-06_*.py` |
| FUN-07 | 0.9 | UT, IT | `typy-check` tests that a generator of each annotation yields and returns its types, and that `send`, `throw` and `close` are only for `Generator`; runtime tests of `send`, `throw` and `close` in each state and of the exception a step hands back to its caller; `tests/conformance/fun/FUN-07_*.py`, CPython's errors of a running, finished or just started generator and the context of an exception raised while one is suspended inside an `except` clause included, also in the sanitizer run |
| FUN-08 | 0.10 | IT | `tests/conformance/fun/FUN-08_*.py` |
| FUN-09 | 0.10 | IT | `tests/conformance/fun/FUN-09_*.py` |
| FUN-10 | 0.10 | IT | `tests/conformance/fun/FUN-10_*.py` |
| FUN-11 | 0.9 | UT, IT, E2E | Runtime tests of closing in each state, of the queue of freed generators and of one that yields again; `tests/conformance/fun/FUN-11_*.py`, with `break`, a temporary, a container, an instance, `with`, nested generators, a variable an exception keeps, `close()` and a generator that yields again, in CPython's order, also in the sanitizer run; `crates/typy-cli/tests/end_to_end/generators.rs` for what standard error shows of an exception while one closes |
| FUN-12 | 1.0 | IT | `tests/conformance/fun/FUN-12_*.py` |
| CLS-01 | 0.2 | IT | `tests/conformance/cls/CLS-01_*.py` |
| CLS-02 | 0.2 | IT | `tests/conformance/cls/CLS-02_*.py` |
| CLS-03 | 0.3 | IT | `tests/conformance/cls/CLS-03_*.py` |
| CLS-04 | 0.4 | IT | `tests/conformance/cls/CLS-04_*` |
| CLS-05 | 0.4 | IT | `tests/conformance/cls/CLS-05_*.py` |
| CLS-06 | 0.4 | IT | `tests/conformance/cls/CLS-06_*.py`; `crates/typy-check/tests/integration/checker.rs`, tests named for methods called through their class |
| CLS-07 | 0.4 | IT | `tests/conformance/cls/CLS-07_*.py` |
| CLS-08 | 0.2 | IT | `tests/conformance/cls/CLS-08_*.py` |
| CLS-09 | 0.4 | IT | `tests/conformance/cls/CLS-09_*.py` |
| CLS-10 | 0.2 | UT, IT | `typy-runtime` type object unit tests; `tests/conformance/cls/CLS-10_*.py` |
| CLS-11 | 0.4 | IT | `tests/conformance/cls/CLS-11_*.py` |
| CLS-12 | 0.2 | UT, IT | `typy-runtime` type object unit tests; `tests/conformance/cls/CLS-12_*.py` |
| CLS-13 | 0.3 | IT | `tests/conformance/cls/CLS-13_*.py` |
| CLS-14 | 0.3 | IT | `tests/conformance/cls/CLS-14_*.py` |
| CLS-15 | 0.5 | IT | `tests/conformance/cls/CLS-15_*.py` |
| CLS-16 | 0.5 | IT | `tests/conformance/cls/CLS-16_*.py` |
| CLS-17 | 0.5 | IT | `tests/conformance/cls/CLS-17_*.py` |
| CLS-20 | 0.5 | IT | `tests/conformance/cls/CLS-20_*.py` |
| CLS-21 | 0.5 | IT | `tests/conformance/cls/CLS-21_*.py` |
| CLS-22 | 0.5 | IT | `tests/conformance/cls/CLS-22_*.py` |
| CLS-23 | 0.5 | IT | `tests/conformance/cls/CLS-23_*.py` |
| CLS-24 | 0.5 | IT | `tests/conformance/cls/CLS-24_*.py` |
| CLS-25 | 0.5 | IT | `tests/conformance/cls/CLS-25_*.py` |
| CLS-26 | 0.5 | IT | `tests/conformance/cls/CLS-26_*.py` |
| CLS-27 | 0.10 | IT | `tests/conformance/cls/CLS-27_*.py` |
| CLS-28 | 1.0 | IT | `tests/conformance/cls/CLS-28_*.py` |
| CLS-29 | 1.0 | IT | `tests/conformance/cls/CLS-29_*.py` |
| CLS-30 | 1.0 | IT | `tests/conformance/cls/CLS-30_*.py` |
| CLS-31 | 1.0 | IT | `tests/conformance/cls/CLS-31_*.py` |
| CLS-32 | 1.0 | IT | `tests/conformance/cls/CLS-32_*.py` |
| CLS-33 | 1.0 | IT | `tests/conformance/cls/CLS-33_*.py` |
| CLS-34 | 1.0 | IT | `tests/conformance/cls/CLS-34_*.py` |
| CLS-35 | 1.0 | IT | `tests/conformance/cls/CLS-35_*.py` |
| CLS-36 | 1.0 | IT | `tests/conformance/cls/CLS-36_*.py` |
| CLS-37 | 1.0 | IT | `tests/conformance/cls/CLS-37_*.py` |
| CLS-18 | 0.10 | IT | `tests/conformance/cls/CLS-18_*.py` |
| CLS-19 | 0.10 | IT | `tests/conformance/cls/CLS-19_*.py` |
| GEN-01 | 0.4 | IT | `tests/conformance/gen/GEN-01_*.py`; `crates/typy-check/tests/integration/generics.rs` |
| GEN-02 | 0.4 | IT | `tests/conformance/gen/GEN-02_*.py` |
| GEN-03 | 0.4 | IT | `tests/conformance/gen/GEN-03_*.py` |
| GEN-04 | 0.4 | IT | `tests/conformance/gen/GEN-04_*.py` |
| GEN-05 | 0.4 | IT | `tests/conformance/gen/GEN-05_*.py` |
| GEN-06 | 1.0 | IT | `tests/conformance/gen/GEN-06_*.py` |
| GEN-07 | 0.5 | IT | `tests/conformance/gen/GEN-07_*`; `crates/typy-check/tests/integration/generics.rs` |
| GEN-08 | 0.5 | IT | `tests/conformance/gen/GEN-08_*`; `crates/typy-check/tests/integration/generics.rs` |
| GEN-09 | 0.5 | IT | `tests/conformance/gen/GEN-09_*.py`; `crates/typy-check/tests/integration/generics.rs` |
| GEN-10 | 0.5 | IT | `tests/conformance/gen/GEN-10_*`; `crates/typy-check/tests/integration/protocols.rs` |
| GEN-11 | 1.0 | IT | `tests/conformance/gen/GEN-11_*.py`; `crates/typy-check/tests/integration/generics.rs` |
| GEN-12 | 1.0 | IT | `tests/conformance/gen/GEN-12_*.py`; `crates/typy-check/tests/integration/generics.rs` |
| GEN-13 | 1.0 | IT | `tests/conformance/gen/GEN-13_*.py`; `crates/typy-check/tests/integration/protocols.rs` |
| GEN-14 | 1.0 | IT | `tests/conformance/gen/GEN-14_*.py`; `crates/typy-check/tests/integration/generics.rs` |
| GEN-15 | 1.0 | IT | `tests/conformance/gen/GEN-15_*.py` |
| GEN-16 | 1.0 | IT | `tests/conformance/gen/GEN-16_*.py`; `crates/typy-check/tests/integration/generics.rs` |
| GEN-17 | 1.0 | IT | `tests/conformance/gen/GEN-17_*.py`; `crates/typy-check/tests/integration/protocols.rs` |
| EXC-01 | 0.3 | IT | `tests/conformance/exc/EXC-01_*.py` |
| EXC-02 | 0.3 | IT | `tests/conformance/exc/EXC-02_*.py` |
| EXC-03 | 0.3 | Golden | `tests/golden/llvm-ir/`, where each call that may raise is followed by one compare of the exception against null and a branch |
| EXC-04 | 0.3 | IT | `tests/conformance/exc/EXC-04_*.py` |
| EXC-05 | 0.5 | E2E | `crates/typy-cli/tests/end_to_end/library.rs`, tests named for EXC-05: an exception, an `int` result outside `int64_t` and a `SystemExit` that leave an export |
| EXC-06 | 0.3 | IT | `tests/conformance/exc/EXC-06_*.py` |
| EXC-07 | 0.3 | IT | `tests/conformance/exc/EXC-07_*.py` |
| EXC-08 | 0.3 | IT | `tests/conformance/exc/EXC-08_*.py` |
| EXC-09 | 0.3 | IT | `tests/conformance/exc/EXC-09_*.py` |
| EXC-10 | 0.3 | IT | `tests/conformance/exc/EXC-10_*.py` |
| EXC-11 | 0.4 | IT | `tests/conformance/exc/EXC-11_*.py` |
| EXC-12 | 0.3 | IT | `tests/conformance/exc/EXC-12_*.py` |
| EXC-13 | 0.4 | IT | `tests/conformance/exc/EXC-13_*.py` |
| EXC-14 | 0.4 | IT | `tests/conformance/exc/EXC-14_*.py` |
| EXC-15 | 0.5 | IT | `tests/conformance/exc/EXC-15_*.py` |
| EXC-16 | 0.5 | IT | `tests/conformance/exc/EXC-16_*.py` |
| EXC-17 | 1.0 | IT | `tests/conformance/exc/EXC-17_*.py` |
| EXC-18 | 1.0 | IT | `tests/conformance/exc/EXC-18_*.py` |
| EXC-19 | 0.7 | UT, IT | `crates/typy-runtime/src/text_file/os_error.rs` for each system's numbers; `tests/conformance/exc/EXC-19_*.py` on Linux and on the `macos-15` job, against CPython on each |
| MOD-01 | 0.3 | IT, E2E | `tests/conformance/mod/MOD-01_*`; module paths in `crates/typy-cli/tests/end_to_end/cli.rs`, tests named for MOD-01 |
| MOD-02 | 0.3 | IT | `tests/conformance/mod/MOD-02_*.py` |
| MOD-03 | 0.3 | IT | `tests/conformance/mod/MOD-03_*.py` |
| MOD-04 | 0.3 | IT | `tests/conformance/mod/MOD-04_*.py` |
| MOD-05 | 0.3 | IT | `tests/conformance/mod/MOD-05_*.py` |
| MOD-06 | 0.3 | IT | `crates/typy-check/tests/integration/lazy_imports.rs`; `crates/typy-check/src/subset/tests.rs`, tests named for lazy imports; `tests/conformance/mod/MOD-06_*`, at floor 3.15 |
| MOD-07 | 0.3 | IT | `crates/typy-check/tests/integration/lazy_imports.rs` and `crates/typy-driver/tests/integration/lazy_imports.rs`, whose expected output is CPython 3.15's: CPython 3.12 to 3.14 ignore `__lazy_modules__`, so a conformance test of it is invalid below floor 3.15 (ADR-0008) |
| MOD-08 | 0.3 | IT | `crates/typy-driver/tests/integration/lazy_imports.rs`; `tests/conformance/mod/MOD-08_*`, at floor 3.15 |
| MOD-09 | 0.3 | IT | `crates/typy-check/tests/integration/sys.rs` for each switch, as an attribute and in `from sys import`; `tests/conformance/mod/MOD-09_*.py` |
| MOD-10 | 0.3 | IT | `tests/conformance/mod/MOD-10_*.py` |
| MOD-11 | 0.4 | IT | `tests/conformance/mod/MOD-11_*` |
| MOD-12 | 0.5 | IT | `tests/conformance/mod/MOD-12_*` |
| MOD-13 | 0.5 | IT | `tests/conformance/mod/MOD-13_*` |
| MOD-14 | 1.0 | IT | `tests/conformance/mod/MOD-14_*` |
| MOD-15 | 1.0 | IT | `tests/conformance/mod/MOD-15_*` |
| LIB-01 | 0.1 | IT | `tests/conformance/lib/LIB-01_*.py` |
| LIB-02 | 0.2 | IT | `tests/conformance/lib/LIB-02_*.py` |
| LIB-03 | 0.2 | IT | `tests/conformance/lib/LIB-03_*.py` |
| LIB-04 | 0.3 | UT, IT, E2E | `crates/typy-runtime/src/sys/tests.rs` for decoding the arguments and each exit status; `crates/typy-runtime/src/text_file/tests.rs` for chunked reading, newline translation, decode and encode errors, buffered writing, each `OSError` and files on disk; `crates/typy-check/src/open.rs` for each mode and encoding name; `crates/typy-check/tests/integration/sys.rs` and `tests/open.rs` for each refusal; `crates/typy-cli/tests/end_to_end/sys.rs` for real arguments, piped standard input and the exit status, and `tests/files.rs` for what a program leaves on disk when it ends; `tests/conformance/lib/LIB-04_*.py` |
| LIB-05 | 0.3 | UT, IT | `crates/typy-runtime/src/math/tests.rs` and `tests/more.rs`, tables of CPython 3.15's results bit for bit; `crates/typy-check/tests/integration/math.rs` for each refusal and the floor of each name; `tests/conformance/lib/LIB-05_*.py` |
| LIB-06 | 0.4 | IT | `tests/conformance/lib/LIB-06_*.py` |
| LIB-07 | 1.0 | IT | `tests/conformance/lib/LIB-07_*.py` |
| LIB-08 | 0.3 | IT | `tests/conformance/lib/LIB-08_*.py` |
| LIB-09 | 0.1 | IT | `tests/conformance/lib/LIB-09_*.py` |
| LIB-10 | 0.3 | UT, IT | `crates/typy-runtime/src/format/percent/tests.rs`, a table of CPython 3.12's results; `crates/typy-check/tests/integration/percent.rs` for each refusal message; `tests/conformance/lib/LIB-10_*.py` |
| LIB-11 | 0.4 | IT | `tests/conformance/lib/LIB-11_*.py` |
| LIB-12 | 0.4 | IT | `tests/conformance/lib/LIB-12_*.py` |
| LIB-15 | 0.5 | IT | `tests/conformance/lib/LIB-15_*.py` |
| LIB-16 | 0.5 | IT | `tests/conformance/lib/LIB-16_*.py` |
| LIB-17 | 0.5 | IT | `tests/conformance/lib/LIB-17_*.py` |
| LIB-18 | 1.0 | IT | `tests/conformance/lib/LIB-18_*.py` |
| LIB-19 | 1.0 | IT | `tests/conformance/lib/LIB-19_*.py` |
| LIB-20 | 0.9 | UT, IT | `typy-check` tests of each abstract collection's members and operations; `tests/conformance/lib/LIB-20_*.py` |
| LIB-21 | 1.0 | IT | `tests/conformance/lib/LIB-21_*.py` |
| LIB-22 | 0.7 | UT, IT | `crates/typy-source/src/settings.rs` for `sys.platform` of each operating system; `crates/typy-driver/tests/integration/pipeline.rs` for the platform each target gives; `tests/conformance/lib/LIB-22_*.py` on Linux and on the `macos-15` job |
| LIB-13 | 0.4 | IT | `tests/conformance/lib/LIB-13_*.py` |
| LIB-14 | 0.4 | IT | `tests/conformance/lib/LIB-14_*.py` |
| MEM-01 | 0.1 | UT, IT | `typy-runtime` unit tests; `tests/conformance/mem/MEM-01_*.py` under sanitizers |
| MEM-02 | 0.2 | UT, IT | `typy-runtime` list unit tests; `tests/conformance/dat/DAT-07_*.py`, which name MEM-02, also under the sanitizers |
| MEM-03 | 0.5 | UT, IT | `crates/typy-tyir/src/escape/tests.rs` for each way an instance escapes; `crates/typy-runtime/src/instance/tests.rs` for a frame block made, reused and released; `crates/typy-lower/tests/integration/frames.rs` for the instances programs hold in frames; `crates/typy-codegen/tests/codegen.rs` for the block and the release before each return; `tests/conformance/mem/MEM-03_*.py` under sanitizers |
| MEM-04 | 0.2 | UT, IT | `typy-lower` lowering tests of the counting; `tests/conformance/mem/MEM-04_*.py` under sanitizers |
| MEM-05 | 1.0 | UT, IT | `typy-runtime` unit tests; `tests/conformance/mem/MEM-05_*.py` under sanitizers |
| MEM-06 | 1.0 | UT, IT | `typy-runtime` unit tests; `tests/conformance/mem/MEM-06_*.py` under sanitizers |
| MEM-07 | 1.0 | UT, IT | `typy-runtime` unit tests; `tests/conformance/mem/MEM-07_*.py` under sanitizers |
| MEM-08 | 0.5 | UT, IT, E2E | `crates/typy-source/src/settings_file/allocator/tests.rs` for each wrong `[allocator]` table; `crates/typy-codegen/tests/codegen.rs` for the hooks; `crates/typy-build/src/linker.rs` for the library's place among the arguments; `crates/typy-cli/tests/end_to_end/allocator.rs` for a program built with a counting allocator, one that has no memory, and a library the linker cannot find |
| MEM-10 | 1.0 | UT, IT | `crates/typy-tyir/src/escape/tests.rs`; `tests/conformance/mem/MEM-10_*.py` under sanitizers |
| MEM-11 | 0.8 | UT, IT | `typy-codegen` tests of the order of each layout and of every descriptor's offsets against LLVM's; the conformance programs, whose output shows the declared order |
| MEM-09 | 0.3 | UT, IT | `crates/typy-lower/tests/integration/moves.rs` for each store that moves and each read that must not; `crates/typy-runtime/src/moved_tests.rs` for the counts of each moved store; `tests/conformance/mem/MEM-09_*.py` under sanitizers |
| FFI-01 | 0.5 | UT, IT | `crates/typy-check/src/check/ctypes_declarations/tests.rs` for each way of naming `ctypes` and loading a library; `tests/conformance/ffi/FFI-01_*.py` against CPython 3.12 to 3.15 |
| FFI-02 | 0.5 | UT, IT | `crates/typy-check/src/check/ctypes_declarations/tests.rs` for each E0430 of a declaration; `tests/conformance/ffi/FFI-02_*.py` for calls declared in time, too early, without a `restype`, and with an argument of the wrong type |
| FFI-03 | 0.5 | UT, IT | `crates/typy-check/src/ctypes.rs` and `ctypes/structure.rs` for the types, their classes, names, sizes, structure layouts and conversions; `crates/typy-check/src/check/ctypes_declarations/tests.rs` for `POINTER`, names bound to a type and structure classes of this module and of others; `crates/typy-check/tests/integration/ctypes.rs` for each E0404, E0402, E0412 and E0401 of an object and each argument a pointer takes; `crates/typy-tyir/src/library.rs` and `crates/typy-lower/tests/integration/foreign.rs` for the class table, pointer parameters and structures by value; `crates/typy-codegen/src/structure_abi.rs` for the register class of each eight bytes of a structure and the registers left; `crates/typy-runtime/src/foreign.rs` for the runtime's conversions and `ctypes.ArgumentError`; `crates/typy-runtime/src/cdata/tests.rs` for scalars, pointers that keep their target, null pointers, checked array indexes and the `bytes` an object keeps for its memory; `tests/conformance/ffi/FFI-03_*.py` for every scalar type's wrap-around, widening, rounding and null, and for objects passed by reference, pointers, arrays and structures, against CPython 3.12 to 3.15, also under the sanitizers; `crates/typy-cli/tests/end_to_end/structures.rs` for structures passed by value in each register class and in memory, against CPython through a C library the test builds |
| FFI-04 | 0.5 | UT, E2E | `crates/typy-source/src/settings_file/libraries.rs` and `settings.rs` for `library-paths` and `[libraries]`; `crates/typy-build/src/library_argument.rs` for the linker's argument of each library on each target; `crates/typy-cli/tests/end_to_end/library.rs`, tests named for FFI-04: a program that imports a TyPy library's binding, linked dynamically and statically, and a library the linker cannot find |
| FFI-05 | 0.5 | UT, IT, E2E | `crates/typy-check/tests/integration/exports.rs` for each name `__all__` lists and each E0429; `crates/typy-tyir/tests/integration/library.rs` for the verifier and the text form; `crates/typy-codegen/tests/codegen.rs` and `src/header.rs` for the wrappers and the header; `crates/typy-runtime/src/int/fixed.rs` for the conversions; `crates/typy-cli/tests/end_to_end/library.rs`, tests named for FFI-05, also comparing the refused names with what the runtime archive calls |
| FFI-08 | 0.5 | UT, E2E | `crates/typy-codegen/src/binding.rs` for the binding and the stub; `crates/typy-cli/tests/end_to_end/library.rs`, tests named for FFI-08 and FFI-04: CPython and a TyPy program call a library through its binding, and CPython 3.12 to 3.15 parse the binding and the stub |
| FFI-09 | 1.0 | IT | `tests/conformance/ffi/FFI-09_*.py` |
| FFI-10 | 1.0 | UT, IT | `crates/typy-check/tests/integration/ctypes.rs` for each E0201 of a `ctypes` use planned for 1.0; `tests/conformance/ffi/FFI-10_*.py` |
| FFI-11 | 1.0 | UT, IT | `crates/typy-check/src/check/ctypes_declarations/tests.rs` and `crates/typy-check/tests/integration/ctypes.rs` for each E0201 of a structure form planned for 1.0; `tests/conformance/ffi/FFI-11_*.py` |
| FFI-12 | 0.6 | UT, E2E | `crates/typy-codegen/src/structure_abi.rs` for each target's classification against clang 19's lowering of the same C structures; `crates/typy-cli/tests/end_to_end/structures.rs` run for each target, under `qemu-aarch64` for AArch64, against CPython on x86_64 |
| FFI-13 | 0.7 | UT, E2E | `crates/typy-codegen/src/foreign/stack_argument.rs` for the registers arguments take and each type's width and attribute on the stack; `crates/typy-codegen/tests/codegen.rs` for the arguments of one call on each target; `crates/typy-cli/tests/end_to_end/structures.rs` calling C functions with more narrow arguments than registers, on each architecture and on the `macos-15` job; the FFI-03 conformance test of integer types, whose C functions are wider than their declarations |
| FFI-14 | 0.7 | UT, IT | `crates/typy-check/src/check/ctypes_declarations/tests.rs` for a library without a name; `tests/conformance/ffi/FFI-14_*.py` on Linux and on the `macos-15` job, and the tests of FFI-02 and FFI-03, which call the C and maths libraries through `CDLL(None)` |
| FFI-06 | 1.0 | IT | `tests/conformance/ffi/FFI-06_*.py` |
| FFI-07 | 1.0 | IT | `tests/conformance/ffi/FFI-07_*.py` |
| DIA-01 | 0.1 | UT, IT | `typy-diagnostics` unit tests; `tests/golden/diagnostics/DIA-01_*` |
| DIA-02 | 0.1 | UT, IT | `typy-diagnostics` unit tests; `tests/golden/diagnostics/DIA-02_*` |
| DIA-03 | 0.1 | UT, IT | `typy-diagnostics` unit tests; `tests/golden/diagnostics/DIA-03_*` |
| DIA-04 | 0.1 | UT, IT | `typy-diagnostics` unit tests; `tests/golden/diagnostics/DIA-04_*` |
| DIA-05 | 0.3 | UT, E2E | `crates/typy-diagnostics/src/render_json.rs` unit and property tests; `crates/typy-cli/tests/end_to_end/cli.rs`, tests named for DIA-05 |
| DIA-06 | 0.1 | UT | `typy-diagnostics` unit tests of the code registry |
| DIA-07 | 0.3 | E2E | `crates/typy-cli/tests/end_to_end/cli.rs`, tests named for DIA-07 |
| DIA-08 | 0.3 | UT | `crates/typy-source/src/parse/tests.rs` for W0101; `crates/typy-check/src/subset/tests.rs` for W0301 |
| DOC-01 | 0.2 | Static | `make docs-lint`; review against the release |
| DOC-02 | 0.2 | Static, IT | `make docs-lint`; every rule ID on the pages is checked to exist by `make traceability`; samples by DOC-04 |
| DOC-03 | 0.2 | IT | `crates/typy-conformance/tests/integration/documentation.rs` lists every active code of the registry and fails when `docs/diagnostics/troubleshooting.md` has no section for one |
| DOC-04 | 0.2 | IT | `crates/typy-conformance/tests/integration/documentation.rs` extracts every `python` sample from the user documentation, every page under `docs/` outside the engineering documents' directories, with the outcome its `typy-expect` comment states, and judges it as a conformance test: a rejected sample by its code and place, an accepted one by running it and comparing with CPython |
| DOC-05 | 0.6 | Static, IT | `make docs`, which `make check` runs: `scripts/build-docs.sh` builds the site with Zensical in strict mode, which fails on a link to a page or an anchor that does not exist; then `scripts/check-docs-nav.py` fails on a page under `docs/` that the `nav` of `mkdocs.yml` leaves out and on a `nav` entry with no page; `.github/workflows/pages.yml` publishes the build of each release |
| NFR-01 | 0.1 | IT | `crates/typy-driver/tests/integration/size.rs`: hello world, `--release-small`, linked statically with musl, run, and under 100 KB (75 KB in 0.1, 17 KB stripped) |
| NFR-02 | 0.1 | Static | `make runtime-size`, `scripts/check-runtime-size.py`: a hello world built with `--release-small` and linked for each target with a link map; the runtime sections linked are under 48 KB, and under 32 KB less the large-integer sections (10.3 KB and 10.3 KB in 0.2, ADR-0009) |
| NFR-03 | 1.0 | Benchmark | `tests/bench/` suite against its C versions, geometric mean within 1.5 times |
| NFR-04 | 1.0 | Benchmark | `tests/bench/build_time.py`: debug build of a 10,000-line project, in one file and in 200 modules, under 2 s; checking 200 modules at most 6 times as long as 50 |
| NFR-05 | 1.0 | Benchmark | `tests/bench/build_time.py`: release build of both 10,000-line projects under 20 s |
| NFR-06 | 0.1 | E2E | `crates/typy-conformance/tests/integration/determinism.rs`: every accepted conformance program built twice, in debug and in release mode, and the executables compared byte for byte |
| NFR-07 | 0.1 | IT | `tests/sanitized.rs`: the conformance suite with the runtime under AddressSanitizer; `make miri`; `make fuzz` |
| NFR-08 | 0.1 | IT | `tests/sanitized.rs`: the conformance suite under LeakSanitizer |
| NFR-09 | 0.1 | Fuzz | `make fuzz`, target `stages` over parsing, checking and lowering; 30 minutes with zero crashes before a release |
| NFR-10 | 0.1 | IT | Coverage report of `typy-conformance`: an accepted and a rejected test per requirement |
| NFR-11 | 0.1 | CI gate | `make coverage-branches`, run by `make check`: `cargo llvm-cov` with the pinned nightly over the compiler and runtime crates, with subprocesses, at least 95 percent of lines and of branches (97.6 and 95.1 in 0.1) |
| NFR-12 | 0.2 | UT | `typy-runtime` layout test: `size_of::<ObjectHeader>() == 8` |
| NFR-13 | 0.2 | UT | `typy-runtime` list test: a million `int` values appended one at a time; the list and its buffer take at most 8.5 MB |
| NFR-14 | 1.0 | Benchmark | `tests/bench/build_time.py`: peak memory of each build within 1 GB |
| NFR-15 | 0.1 | IT | Conformance suite: debug and release runs compared |
| NFR-16 | 0.6 | IT | `crates/typy-conformance/tests/integration/other_architecture.rs`: every accepted conformance test built for the other architecture, in debug and release mode, and run under its user-mode emulator, `qemu-aarch64` on an x86_64 host and `qemu-x86_64` on an arm64 host, each run compared with CPython's output, except a test marked `typy-output: architecture-dependent`, whose output CPython itself gives otherwise on each architecture; on the arm64 CI runner the suite's own test runs the AArch64 builds natively against CPython on AArch64, which checks those too |
| NFR-17 | 0.7 | IT | The conformance suite on the `macos-15` CI job, each accepted test compared with CPython 3.12 to 3.15 for macOS from python-build-standalone; a test whose header says `# typy-system: linux` or `macos`, because it names a library only that system has, runs on that system alone, which `crates/typy-conformance/tests/integration/verdicts.rs` checks |
| NFR-18 | 0.8 | UT | `typy-codegen` layout test of instances that mix every kind of attribute |
| NFR-19 | 0.8 | UT, IT | `typy-check` tests that reject a function, a `lambda` or a method where its parameter or result types do not fit the expected `Callable`; the sanitizer run of every conformance test of FUN-04, FUN-05, SMT-09 and EXP-14 |
| NFR-20 | 0.9 | UT | `typy-codegen` test of a generator object's layout; a runtime test that counts the allocations of a loop over a generator |
| C-01 | 0.1 | CI gate | Workspace builds with the pinned Rust toolchain only |
| C-02 | 0.1 | CI gate | `scripts/check-pins.sh`: `ruff_python_parser` pinned with `=` in `Cargo.toml` |
| C-03 | 0.1 | UT | `typy-codegen` test: the linked LLVM major version is 23 |
| C-04 | 0.1 | CI gate | `cargo deny check licenses` |
| C-05 | 0.1 | E2E | E2E tests run in a container with no network |
