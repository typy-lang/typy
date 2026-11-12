# TyPy development guide

| Field | Value |
| --- | --- |
| Document | TyPy development guide |
| Version | 0.16 |
| Status | Draft |
| Owner | typy-lang contributors |
| Last updated | 2026-11-12 |

## 1. Purpose

This guide tells you how to set up a working copy of TyPy, which commands
to run day to day, how the repository is laid out, how changes are checked,
committed and released. It builds on the approved
[test plan](../testing/test-plan.md) and
[detailed design](../design/detailed-design.md).

## 2. Set-up

Every tool runs in the containers defined in `.devcontainer/compose.yaml`.
Your machine needs only:

| Tool | Version |
| --- | --- |
| Docker Engine | 24 or later |
| Docker Compose | v2 |
| GNU Make | 4 or later |
| Git | 2.34 or later |

Then, from the repository root:

```sh
make hooks
make dev
make check
```

`make hooks` points Git at `.githooks`, so every commit runs the quality
gate. `make dev` builds the dev image the first time, which downloads Rust,
the cargo tools and Node, and starts the `dev` service. `make check` runs
the gate once to show the set-up works.

To work in VS Code, open the folder and run "Dev Containers: Reopen in
Container". VS Code attaches to the same `dev` service.

## 3. Everyday commands

| Command | What it does |
| --- | --- |
| `make dev` | Starts the `dev` service, building its image the first time |
| `make shell` | Opens a shell in the `dev` service |
| `make build` | Builds the compiler, `<target>/debug/typy`, after `make runtime` |
| `make runtime` | Builds the runtime archive for `x86_64-unknown-linux-gnu` and `x86_64-unknown-linux-musl` with `scripts/build-runtime.sh`, and stages each in `<target>/lib/typy/<triple>/`, where a compiler in `<target>/debug` or `<target>/release` finds it; `make test` and `make coverage` run it first |
| `make bless` | Rewrites the golden files in `tests/golden/` from the current compiler; review the diff before committing |
| `make miri` | Runs the runtime's unit tests under Miri with the pinned nightly toolchain, in about 8 minutes |
| `make fuzz` | Fuzzes the source, checking and lowering stages for `FUZZ_SECONDS` seconds, 60 by default; a crash leaves its input in `fuzz/artifacts/` |
| `make runtime-size` | Links a hello world for each target and fails when the runtime it links is over the size budgets of NFR-02 |
| `make mutants MUTANTS_PACKAGE=typy-check` | Runs `cargo-mutants` on one package in a container limited to 6 GB of memory with no swap, 4 CPUs and no network, made from an image of the dev container that is removed afterwards; the report goes to `.tmp/mutants/PACKAGE/` |
| `make bench` | Builds the compiler in release mode and times it on generated 10,000-line projects, in one file and in 200 modules, against the budgets of NFR-04, NFR-05 and NFR-14, with a check that checking time grows linearly with the modules |
| `make stdlib-names` | Writes `crates/typy-source/src/stdlib_names.rs`, the standard library module names of CPython 3.12 to 3.15, which the loader leaves to the checker (LIB-08); `make check` runs `make stdlib-names-check` |
| `make check` | Runs the whole quality gate: formatting, Clippy, the tests with line and branch coverage, the four documentation checks below, the Unicode table check and the runtime size check |
| `make fmt` | Formats the Rust code |
| `make lint` | Runs Clippy on every crate and test, with warnings as errors |
| `make type` | Type checks every crate and test |
| `make test` | Runs the unit tests with `cargo nextest` |
| `make coverage` | Runs the tests with coverage on the stable toolchain and fails below 95 percent of lines |
| `make coverage-branches` | Runs the tests with coverage on the pinned nightly toolchain and fails below 95 percent of lines or of branches (NFR-11); `make check` runs it |
| `make test-int` | Runs the integration tests: the conformance suite and its sanitizer run, the golden files and the pipeline |
| `make test-e2e` | Runs the end-to-end tests of the `typy` executable |
| `make test-all` | Runs `make test` and `make miri` |
| `make docs-lint` | Lints every Markdown file with markdownlint-cli2 |
| `make ascii-check` | Fails on any non-ASCII byte in tracked text files, except rendered SVG |
| `make traceability` | Fails when a requirement has no row in the test plan's matrix, or a document cites an ID that is not defined |
| `make diagrams` | Renders every PlantUML source to the SVG beside it |
| `make diagrams-check` | Renders, then fails when an SVG differs from the one staged |
| `make unicode-tables` | Writes `crates/typy-runtime/src/string/unicode/tables.rs` from the `unicodedata` of CPython 3.15 (OI-15); never edit that file by hand |
| `make unicode-tables-check` | Fails when the committed Unicode tables differ from the ones CPython 3.15 gives |
| `make format-cases` | Writes `crates/typy-runtime/src/format/tests/cases.rs`, the results `format(value, spec)` gives under CPython 3.12 to 3.15 for a table of cases, keeping those all four agree on; never edit that file by hand |
| `make format-cases-check` | Fails when the committed table of `format()` results differs from the one CPython 3.12 to 3.15 give |
| `make image` | Rebuilds the dev image after a change under `.devcontainer/` |
| `make clean` | Stops the containers and removes `.tmp/preview` |

The dev image holds one pinned nightly toolchain, `nightly-2026-11-05`, for
`make miri`, `make fuzz` and the runtime archive built with
AddressSanitizer; every other command uses the stable toolchain of
`rust-toolchain.toml`. The fuzz targets in `fuzz/` are a Cargo workspace of
their own, so the main workspace never needs nightly.

`cargo llvm-cov` measures branch coverage only on a nightly toolchain, so `make
coverage-branches` uses the pinned nightly in a target directory of its own;
profiles from the stable build of `make coverage` would not match its build.
Both leave out `typy-conformance`, which is test code; NFR-11 covers the
compiler and the runtime. The runtime's branches are measured by its unit tests,
because the programs the other tests build link the runtime archive without
instrumentation. `make bench` builds the compiler in release mode and times it
on generated 10,000-line projects against the budgets of NFR-04, NFR-05 and
NFR-14; CI runs it after the release archive. The user documentation in
`docs/user` is Markdown that `make docs-lint` checks, and
`crates/typy-conformance/tests/documentation.rs` judges its samples (DOC-04).

## 4. Repository layout

| Path | Holds |
| --- | --- |
| `.devcontainer/` | The dev image, and the compose file with the `dev` and `diagrams` services |
| `.github/` | The CI, release and code scanning workflows, Dependabot's settings and the pull request template |
| `.githooks/` | The pre-commit hook, which runs `make check` |
| `docs/` | The engineering documents, listed with their status in `docs/index.md` |
| `docs/diagrams/` | PlantUML sources, the shared `style.puml`, and the rendered SVG of each |
| `fuzz/` | The fuzz targets, a Cargo workspace of their own that builds with the pinned nightly |
| `scripts/` | The checks and builds that `make` runs, the release scripts, and the generator of the Unicode tables |
| `tests/conformance/` | The conformance programs, by requirement area |
| `tests/golden/` | The golden programs and the expected text of each stage and diagnostic |
| `dist/` | Release archives that `make dist` builds. Ignored by Git |
| `rust-toolchain.toml` | The Rust version and components every build uses |
| `.tmp/` | Scratch files. Ignored by Git and never committed |

The crates under `crates/` and the tests under `tests/` follow detailed
design section 2.

### 4.1 Organising code

New code follows detailed design section 3.1, so the codebase stays easy
to change as it grows:

- Give each kind of object, or each responsibility, its own module.
- Write a module with children as `name.rs` plus a `name/` folder, never
  `mod.rs`.
- Keep modules private and re-export the public API from the crate root.
- Use the narrowest visibility that works: `pub(super)`, then
  `pub(crate)`, then `pub`.
- Split a source file by responsibility before it passes about 400 lines.
- Put tests that need only the public API in the crate's `tests/` folder,
  and the rest next to the module they test.

## 5. Working on the documents

- A change in behaviour updates the specification, the design, the test
  plan and, once they exist, the user documentation and the changelog, in
  the same commit.
- A new requirement gets the next unused number in its area and a row in
  the test plan's matrix; `make traceability` fails until it has one. An ID
  is never reused or renumbered.
- A decision gets a new record in `docs/adr/`, named
  `NNNN-the-decision-in-words.md`, and a row in `docs/adr/index.md`. An
  accepted record is not rewritten; a later record supersedes it.
- A document's header carries its version, status and date. A change
  raises the version and the date.

### 5.1 Adding a diagram

1. Write `docs/diagrams/<type>-<subject>.puml`, starting with
   `!include style.puml` and a `title`.
2. Run `make diagrams` and open the SVG. Check that nothing overlaps and
   that it reads at the width of the site; split a diagram that does not.
3. Add a row for it to the diagram table in `docs/index.md`.
4. Stage the source and the SVG together.

## 6. Adding a tool

A tool the project needs goes into the dev image or into a service of its
own in `.devcontainer/compose.yaml`, with its version pinned, and gets a
`make` target. Add it as a new layer near the end of the Dockerfile, so the
cached layers before it are kept, then run `make image`. Nothing is
installed on the host.

## 7. Commits

- The quality gate passes before every commit; the pre-commit hook runs it.
- One change per commit, small enough to review alone, with its tests and
  documents.
- The subject says what the change does, in the imperative and in ASCII:
  "Add the test plan and a traceability check to the gate". A short body
  follows only when it adds something the subject cannot.
- A commit names the requirement, issue or decision it relates to, such as
  `MOD-06` or `ADR-0008`, in its body when the subject does not.
- Commits carry no sign-off or attribution lines.

## 8. Continuous integration

CI runs the same `make` targets as a working copy, in the same containers,
so a change that passes `make check` locally passes CI. Every workflow holds
read-only permissions, and only the job that creates a release may write.

| Workflow | Job | Runs on | Does |
| --- | --- | --- | --- |
| `ci.yml` | Quality gate and package | Every pull request | `make check`, then `make dist` |
| `ci.yml` | Miri and fuzzing | On demand, before a release | `make miri`, then `FUZZ_SECONDS=1800 make fuzz`; the inputs that crashed are kept as an artifact |
| `ci.yml` | Dependency review | Every pull request | Fails on a new dependency with a known vulnerability |
| `codeql.yml` | Analyze | Every push to `main`, and weekly | Code scanning of the Rust sources and the workflows |
| `release.yml` | Check and build, then Publish | A tag `vX.Y.Z` | Section 9 |

CI minutes are spent on whole changes. `main` changes only through a pull
request that is up to date and passes the quality gate, so CI does not run
again after the merge, and a change is pushed when it is complete, never as a
series of partial pushes. Dependabot proposes one pull request a month for
the actions and one for the Cargo dependencies of the workspace and of
`fuzz/`. The repository's settings protect
`main`: pull requests only, the quality gate job required, linear history,
squash merges and no force pushes; and the `release` environment requires a
maintainer's approval.

## 9. Releases

1. `make miri` and `FUZZ_SECONDS=1800 make fuzz` pass, locally or through
   a run of `ci.yml` started by hand (NFR-07, NFR-09), and `make mutants`
   has run on each package the test plan names, with every mutant that
   survives either caught by a new test or recorded as equivalent.
2. A commit titled "Prepare X.Y.Z" sets the version in `Cargo.toml`,
   renames the changelog's "Unreleased" section to `[X.Y.Z] - DATE`, and
   opens a new empty "Unreleased" section above it. For a new minor
   release it adds the release to `RELEASES` in
   `crates/typy-conformance/tests/suite.rs`, so the suite requires an
   accepted and a rejected test of every requirement up to it.
3. A tag `vX.Y.Z` on that commit starts `release.yml`.
4. `scripts/check-release.sh` checks that the tag names the version and that
   the changelog has a dated section for it, whose text becomes the release
   notes. The job then runs `make check` and `make dist`.
5. A maintainer approves the `release` environment.
6. The publish job creates the GitHub release with the archive and its
   SHA-256, using the workflow's short-lived token and no long-lived one.
   It then downloads the archive back, checks its SHA-256, unpacks it and
   runs a program with it.

`make dist` builds `dist/typy-X.Y.Z-x86_64-unknown-linux-gnu.tar.gz`: the
`typy` executable in `bin/` and a runtime archive for each target in
`lib/typy/<triple>/`, which is the layout of an installation prefix, with
the licences, README and changelog. The tarball is reproducible, so two
builds of one commit have the same SHA-256. The executable links LLVM
statically and needs glibc 2.39 or later, `libffi`, `zlib`, `zstd` and
`libstdc++`, and a C toolchain to link programs.

A release that breaks compatibility goes out as a release candidate first,
with a version such as `1.0.0-rc.1`, which the workflow marks as a
pre-release.
