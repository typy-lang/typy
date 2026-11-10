# TyPy test plan

| Field | Value |
| --- | --- |
| Document | TyPy test plan |
| Version | 0.22 |
| Status | Draft |
| Owner | typy-lang contributors |
| Last updated | 2026-11-10 |

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
| Golden | The text of one compiler stage for one input, through `--emit` (CLI-04) | `tests/golden/`, run by `crates/typy-driver/tests/golden.rs` | Every commit |
| Integration (IT) | The compiler, runtime and C toolchain together on one program, compared with CPython; the conformance suite | `tests/conformance/<area>/` through `typy-conformance`, which drives `typy-driver` in process | Every commit |
| End-to-end (E2E) | The `typy` executable as a user runs it: arguments, exit status, files written, and the built program run as a subprocess | `crates/typy-cli/tests/cli.rs` | Every commit |
| Fuzz | No input crashes the compiler (NFR-09) | `fuzz/` | 30 minutes before each release; 60 seconds on demand |
| Benchmark | The numbered budgets of NFR-03 to NFR-05, NFR-13 and NFR-14 | `tests/bench/` | Before a release, from 1.0 |
| Mutation | Tests fail when the code they cover is changed | `cargo-mutants` over the compiler crates | Before a release, from 0.2 |

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
`EXP-03_float_division_by_zero_raises.py`.

Two requirements have no rejected file: LEX-01, because a source that is
not UTF-8 would break the repository's ASCII rule, and LEX-09, because a
source over 16 MiB is too large to commit. Unit tests in `typy-source` build
those inputs when they run and check E0102 and E0101. Where a 0.1 program
cannot show the rejection a requirement describes, its rejected test records
what 0.1 does instead: an import is E0201 until imports arrive in 0.3, and a
function used as a truth value is E0201 until functions are values in 1.0.

The test `crates/typy-conformance/tests/suite.rs` runs the suite on every
commit and fails when a test does not pass or a 0.1 requirement of the
matrix lacks either kind of test (NFR-10). The 0.1 corpus holds 45 accepted
and 44 rejected tests and runs in about 20 seconds.

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

The test `crates/typy-conformance/tests/sanitized.rs` builds every accepted
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
| Dev container | `typy-lang-dev`: Rust 1.99.1, the cargo tools, LLVM 23 from apt.llvm.org, musl, Node and markdownlint | UT, golden, E2E, local runs of everything |
| Oracle interpreters | CPython 3.12.16, 3.13.18, 3.14.10 and 3.15.2, exact builds installed with uv 0.13.7 in the dev container, as `python3.12` to `python3.15`, with nothing else installed | Conformance |
| Nightly Rust | One pinned nightly toolchain, for Miri and the sanitizers only | Section 3.5 |
| Diagrams container | PlantUML 1.2026.8, pinned by digest | Documentation gate |

Each release adds its platforms to the CI matrix as section 2.3 of the
specification lists them: Linux x86_64 in 0.1, Linux and macOS AArch64 in
0.5, Windows x86_64 in 1.0.

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
- The benchmark programs and their C versions are committed as source.

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
| CLI-01 | 0.1 | E2E | `crates/typy-cli/tests/cli.rs`, tests named for CLI-01 |
| CLI-02 | 0.1 | E2E | `crates/typy-cli/tests/cli.rs`, tests named for CLI-02 |
| CLI-03 | 0.1 | E2E | `crates/typy-cli/tests/cli.rs`, tests named for CLI-03 |
| CLI-04 | 0.1 | E2E, golden | `crates/typy-cli/tests/cli.rs`, tests named for CLI-04; stage golden files in `tests/golden/` |
| CLI-05 | 0.1 | E2E | `crates/typy-cli/tests/cli.rs`, tests named for CLI-05 |
| CLI-06 | 0.1 | E2E | `crates/typy-cli/tests/cli.rs`, tests named for CLI-06 |
| CLI-07 | 0.3 | E2E | `crates/typy-cli/tests/cli.rs`, tests named for CLI-07 |
| CLI-08 | 0.5 | E2E | `crates/typy-cli/tests/cli.rs`, tests named for CLI-08 |
| CLI-09 | 0.5 | E2E | `crates/typy-cli/tests/cli.rs`, tests named for CLI-09 |
| CLI-10 | 0.3 | E2E | `crates/typy-cli/tests/cli.rs`, tests named for CLI-10 |
| CLI-11 | 0.1 | E2E | `crates/typy-cli/tests/cli.rs`, tests named for CLI-11 |
| CLI-12 | 0.3 | E2E | `crates/typy-cli/tests/cli.rs`, tests named for CLI-12 |
| LEX-01 | 0.1 | IT | `tests/conformance/lex/LEX-01_*.py` |
| LEX-02 | 0.1 | IT | `tests/conformance/lex/LEX-02_*.py` |
| LEX-03 | 0.1 | IT | `tests/conformance/lex/LEX-03_*.py` |
| LEX-04 | 0.1 | IT | `tests/conformance/lex/LEX-04_*.py` |
| LEX-05 | 0.1 | IT | `tests/conformance/lex/LEX-05_*.py` |
| LEX-06 | 0.2 | IT | `tests/conformance/lex/LEX-06_*.py` |
| LEX-07 | 0.2 | IT | `tests/conformance/lex/LEX-07_*.py` |
| LEX-08 | 0.1 | IT | `tests/conformance/lex/LEX-08_*.py` |
| LEX-09 | 0.1 | IT | `tests/conformance/lex/LEX-09_*.py` |
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
| EXP-14 | 1.0 | IT | `tests/conformance/exp/EXP-14_*.py` |
| EXP-15 | 1.0 | IT | `tests/conformance/exp/EXP-15_*.py` |
| EXP-16 | 0.1 | IT | `tests/conformance/exp/EXP-16_*.py` |
| EXP-17 | 1.0 | IT | `tests/conformance/exp/EXP-17_*.py` |
| SMT-01 | 0.1 | IT | `tests/conformance/smt/SMT-01_*.py` |
| SMT-02 | 0.2 | IT | `tests/conformance/smt/SMT-02_*.py` |
| SMT-03 | 0.1 | IT | `tests/conformance/smt/SMT-03_*.py` |
| SMT-04 | 0.3 | IT | `tests/conformance/smt/SMT-04_*.py` |
| SMT-05 | 0.3 | IT | `tests/conformance/smt/SMT-05_*.py` |
| SMT-06 | 0.3 | IT | `tests/conformance/smt/SMT-06_*.py` |
| SMT-07 | 0.3 | IT | `tests/conformance/smt/SMT-07_*.py` |
| SMT-08 | 0.3 | IT | `tests/conformance/smt/SMT-08_*.py` |
| SMT-09 | 1.0 | IT | `tests/conformance/smt/SMT-09_*.py` |
| SMT-10 | 0.4 | IT | `tests/conformance/smt/SMT-10_*.py` |
| SMT-11 | 1.0 | IT | `tests/conformance/smt/SMT-11_*.py` |
| CST-01 | 0.1 | IT | `tests/conformance/cst/CST-01_*.py` |
| CST-02 | 0.1 | IT | `tests/conformance/cst/CST-02_*.py` |
| CST-03 | 0.2 | IT | `tests/conformance/cst/CST-03_*.py` |
| CST-04 | 0.3 | IT | `tests/conformance/cst/CST-04_*.py` |
| CST-05 | 0.3 | IT | `tests/conformance/cst/CST-05_*.py` |
| CST-06 | 0.5 | IT | `tests/conformance/cst/CST-06_*.py` |
| CST-07 | 0.5 | IT | `tests/conformance/cst/CST-07_*.py` |
| CST-08 | 0.3 | IT | `tests/conformance/cst/CST-08_*.py` |
| CST-09 | 0.3 | IT | `tests/conformance/cst/CST-09_*.py` |
| FUN-01 | 0.1 | IT | `tests/conformance/fun/FUN-01_*.py` |
| FUN-02 | 0.1 | IT | `tests/conformance/fun/FUN-02_*.py` |
| FUN-03 | 1.0 | IT | `tests/conformance/fun/FUN-03_*.py` |
| FUN-04 | 1.0 | IT | `tests/conformance/fun/FUN-04_*.py` |
| FUN-05 | 1.0 | IT | `tests/conformance/fun/FUN-05_*.py` |
| FUN-06 | 1.0 | IT | `tests/conformance/fun/FUN-06_*.py` |
| FUN-07 | 1.0 | IT | `tests/conformance/fun/FUN-07_*.py` |
| CLS-01 | 0.2 | IT | `tests/conformance/cls/CLS-01_*.py` |
| CLS-02 | 0.2 | IT | `tests/conformance/cls/CLS-02_*.py` |
| CLS-03 | 0.3 | IT | `tests/conformance/cls/CLS-03_*.py` |
| CLS-04 | 0.4 | IT | `tests/conformance/cls/CLS-04_*.py` |
| CLS-05 | 0.4 | IT | `tests/conformance/cls/CLS-05_*.py` |
| CLS-06 | 0.4 | IT | `tests/conformance/cls/CLS-06_*.py` |
| CLS-07 | 0.4 | IT | `tests/conformance/cls/CLS-07_*.py` |
| CLS-08 | 0.2 | IT | `tests/conformance/cls/CLS-08_*.py` |
| CLS-09 | 0.4 | IT | `tests/conformance/cls/CLS-09_*.py` |
| CLS-10 | 0.2 | UT, IT | `typy-runtime` type object unit tests; `tests/conformance/cls/CLS-10_*.py` |
| CLS-11 | 0.4 | IT | `tests/conformance/cls/CLS-11_*.py` |
| CLS-12 | 0.2 | UT, IT | `typy-runtime` type object unit tests; `tests/conformance/cls/CLS-12_*.py` |
| GEN-01 | 0.4 | IT | `tests/conformance/gen/GEN-01_*.py` |
| GEN-02 | 0.4 | IT | `tests/conformance/gen/GEN-02_*.py` |
| GEN-03 | 0.4 | IT | `tests/conformance/gen/GEN-03_*.py` |
| GEN-04 | 0.4 | IT | `tests/conformance/gen/GEN-04_*.py` |
| GEN-05 | 0.4 | IT | `tests/conformance/gen/GEN-05_*.py` |
| EXC-01 | 0.3 | IT | `tests/conformance/exc/EXC-01_*.py` |
| EXC-02 | 0.3 | IT | `tests/conformance/exc/EXC-02_*.py` |
| EXC-03 | 0.3 | IT | `tests/conformance/exc/EXC-03_*.py` |
| EXC-04 | 0.3 | IT | `tests/conformance/exc/EXC-04_*.py` |
| EXC-05 | 0.5 | IT | `tests/conformance/exc/EXC-05_*.py` |
| MOD-01 | 0.3 | IT | `tests/conformance/mod/MOD-01_*.py` |
| MOD-02 | 0.3 | IT | `tests/conformance/mod/MOD-02_*.py` |
| MOD-03 | 0.3 | IT | `tests/conformance/mod/MOD-03_*.py` |
| MOD-04 | 0.3 | IT | `tests/conformance/mod/MOD-04_*.py` |
| MOD-05 | 0.3 | IT | `tests/conformance/mod/MOD-05_*.py` |
| MOD-06 | 0.3 | IT | `tests/conformance/mod/MOD-06_*.py` |
| MOD-07 | 0.3 | IT | `tests/conformance/mod/MOD-07_*.py` |
| MOD-08 | 0.3 | IT | `tests/conformance/mod/MOD-08_*.py` |
| MOD-09 | 0.3 | IT | `tests/conformance/mod/MOD-09_*.py` |
| LIB-01 | 0.1 | IT | `tests/conformance/lib/LIB-01_*.py` |
| LIB-02 | 0.2 | IT | `tests/conformance/lib/LIB-02_*.py` |
| LIB-03 | 0.2 | IT | `tests/conformance/lib/LIB-03_*.py` |
| LIB-04 | 0.3 | IT | `tests/conformance/lib/LIB-04_*.py` |
| LIB-05 | 0.3 | IT | `tests/conformance/lib/LIB-05_*.py` |
| LIB-06 | 0.4 | IT | `tests/conformance/lib/LIB-06_*.py` |
| LIB-07 | 1.0 | IT | `tests/conformance/lib/LIB-07_*.py` |
| LIB-08 | 0.3 | IT | `tests/conformance/lib/LIB-08_*.py` |
| LIB-09 | 0.1 | IT | `tests/conformance/lib/LIB-09_*.py` |
| LIB-10 | 0.3 | IT | `tests/conformance/lib/LIB-10_*.py` |
| MEM-01 | 0.1 | UT, IT | `typy-runtime` unit tests; `tests/conformance/mem/MEM-01_*.py` under sanitizers |
| MEM-02 | 0.2 | UT, IT | `typy-runtime` list unit tests; `tests/conformance/dat/DAT-07_*.py`, which name MEM-02, also under the sanitizers |
| MEM-03 | 0.4 | UT, IT | `typy-runtime` unit tests; `tests/conformance/mem/MEM-03_*.py` under sanitizers |
| MEM-04 | 0.2 | UT, IT | `typy-lower` lowering tests of the counting; `tests/conformance/mem/MEM-04_*.py` under sanitizers |
| MEM-05 | 1.0 | UT, IT | `typy-runtime` unit tests; `tests/conformance/mem/MEM-05_*.py` under sanitizers |
| MEM-06 | 0.3 | UT, IT | `typy-lower` lowering tests of the counting; `tests/conformance/mem/MEM-06_*.py` under sanitizers |
| MEM-06 | 1.0 | UT, IT | `typy-runtime` unit tests; `tests/conformance/mem/MEM-06_*.py` under sanitizers |
| MEM-07 | 1.0 | UT, IT | `typy-runtime` unit tests; `tests/conformance/mem/MEM-07_*.py` under sanitizers |
| MEM-08 | 0.5 | UT, IT | `typy-runtime` unit tests; `tests/conformance/mem/MEM-08_*.py` under sanitizers |
| FFI-01 | 0.5 | IT | `tests/conformance/ffi/FFI-01_*.py` |
| FFI-02 | 0.5 | IT | `tests/conformance/ffi/FFI-02_*.py` |
| FFI-03 | 0.5 | IT | `tests/conformance/ffi/FFI-03_*.py` |
| FFI-04 | 0.5 | IT | `tests/conformance/ffi/FFI-04_*.py` |
| FFI-05 | 0.5 | IT | `tests/conformance/ffi/FFI-05_*.py` |
| FFI-06 | 1.0 | IT | `tests/conformance/ffi/FFI-06_*.py` |
| FFI-07 | 1.0 | IT | `tests/conformance/ffi/FFI-07_*.py` |
| DIA-01 | 0.1 | UT, IT | `typy-diagnostics` unit tests; `tests/golden/diagnostics/DIA-01_*` |
| DIA-02 | 0.1 | UT, IT | `typy-diagnostics` unit tests; `tests/golden/diagnostics/DIA-02_*` |
| DIA-03 | 0.1 | UT, IT | `typy-diagnostics` unit tests; `tests/golden/diagnostics/DIA-03_*` |
| DIA-04 | 0.1 | UT, IT | `typy-diagnostics` unit tests; `tests/golden/diagnostics/DIA-04_*` |
| DIA-05 | 0.3 | UT, IT | `typy-diagnostics` unit tests; `tests/golden/diagnostics/DIA-05_*` |
| DIA-06 | 0.1 | UT | `typy-diagnostics` unit tests of the code registry |
| DIA-07 | 0.3 | UT, IT | `typy-diagnostics` unit tests; `tests/golden/diagnostics/DIA-07_*` |
| DOC-01 | 0.2 | Static | `make docs-lint`; review against the release |
| DOC-02 | 0.2 | Static, IT | `make docs-lint`; every rule ID on the pages is checked to exist by `make traceability`; samples by DOC-04 |
| DOC-03 | 0.2 | UT | A test lists every active code of the registry and fails when the troubleshooting page has no section for one |
| DOC-04 | 0.2 | IT | A test extracts every Python sample from `docs/user/`, checks it with `Pipeline::check` and, for an accepted sample, runs it and compares with CPython |
| NFR-01 | 0.1 | IT | `crates/typy-driver/tests/size.rs`: hello world, `--release-small`, linked statically with musl, run, and under 100 KB (75 KB in 0.1, 17 KB stripped) |
| NFR-02 | 0.1 | Static | `make runtime-size`, `scripts/check-runtime-size.py`: a hello world built with `--release-small` and linked for each target with a link map; the runtime sections linked are under 48 KB, and under 32 KB less the large-integer sections (10.3 KB and 10.3 KB in 0.2, ADR-0009) |
| NFR-03 | 1.0 | Benchmark | `tests/bench/` suite against its C versions, geometric mean within 1.5 times |
| NFR-04 | 1.0 | Benchmark | `tests/bench/build_time`: debug build of the 10,000-line project under 2 s |
| NFR-05 | 1.0 | Benchmark | `tests/bench/build_time`: release build of the 10,000-line project under 20 s |
| NFR-06 | 0.1 | E2E | Determinism job: every conformance program built twice, bytes compared |
| NFR-07 | 0.1 | IT | `tests/sanitized.rs`: the conformance suite with the runtime under AddressSanitizer; `make miri`; `make fuzz` |
| NFR-08 | 0.1 | IT | `tests/sanitized.rs`: the conformance suite under LeakSanitizer |
| NFR-09 | 0.1 | Fuzz | `make fuzz`, target `stages` over parsing, checking and lowering; 30 minutes with zero crashes before a release |
| NFR-10 | 0.1 | IT | Coverage report of `typy-conformance`: an accepted and a rejected test per requirement |
| NFR-11 | 0.1 | CI gate | `make coverage-branches`, run by `make check`: `cargo llvm-cov` with the pinned nightly over the compiler and runtime crates, with subprocesses, at least 95 percent of lines and of branches (97.6 and 95.1 in 0.1) |
| NFR-12 | 0.2 | UT | `typy-runtime` layout test: `size_of::<ObjectHeader>() == 8` |
| NFR-13 | 0.2 | UT | `typy-runtime` list test: a million `int` values appended one at a time; the list and its buffer take at most 8.5 MB |
| NFR-14 | 1.0 | Benchmark | `tests/bench/build_time`: peak memory of the 10,000-line build within 1 GB |
| NFR-15 | 0.1 | IT | Conformance suite: debug and release runs compared |
| C-01 | 0.1 | CI gate | Workspace builds with the pinned Rust toolchain only |
| C-02 | 0.1 | CI gate | `scripts/check-pins.sh`: `ruff_python_parser` pinned with `=` in `Cargo.toml` |
| C-03 | 0.1 | UT | `typy-codegen` test: the linked LLVM major version is 23 |
| C-04 | 0.1 | CI gate | `cargo deny check licenses` |
| C-05 | 0.1 | E2E | E2E tests run in a container with no network |
