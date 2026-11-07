# TyPy development guide

| Field | Value |
| --- | --- |
| Document | TyPy development guide |
| Version | 0.6 |
| Status | Draft |
| Owner | typy-lang contributors |
| Last updated | 2026-11-07 |

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
| `make runtime` | Builds the runtime archive and stages it in `<target>/lib/typy/<triple>/`, where a compiler in `<target>/debug` or `<target>/release` finds it; `make test` and `make coverage` run it first |
| `make check` | Runs the whole quality gate: formatting, Clippy, the tests with coverage, the four documentation checks below, and the Unicode table check |
| `make fmt` | Formats the Rust code |
| `make lint` | Runs Clippy on every crate and test, with warnings as errors |
| `make type` | Type checks every crate and test |
| `make test` | Runs the unit tests with `cargo nextest` |
| `make coverage` | Runs the tests with coverage and fails below 95 percent of lines |
| `make docs-lint` | Lints every Markdown file with markdownlint-cli2 |
| `make ascii-check` | Fails on any non-ASCII byte in tracked text files, except rendered SVG |
| `make traceability` | Fails when a requirement has no row in the test plan's matrix, or a document cites an ID that is not defined |
| `make diagrams` | Renders every PlantUML source to the SVG beside it |
| `make diagrams-check` | Renders, then fails when an SVG differs from the one staged |
| `make unicode-tables` | Writes `crates/typy-runtime/src/string/unicode/tables.rs` from the `unicodedata` of CPython 3.15 (OI-15); never edit that file by hand |
| `make unicode-tables-check` | Fails when the committed Unicode tables differ from the ones CPython 3.15 gives |
| `make image` | Rebuilds the dev image after a change under `.devcontainer/` |
| `make clean` | Stops the containers and removes `.tmp/preview` |

`cargo llvm-cov` measures branch coverage only on a nightly toolchain, so
`make coverage` checks lines, and the pinned nightly job of the test plan
measures branches. The targets `test-int`, `test-e2e`, `test-all`,
`bench`, `build` and `docs` are added with the crates that need them,
planned for 0.1.

## 4. Repository layout

| Path | Holds |
| --- | --- |
| `.devcontainer/` | The dev image, and the compose file with the `dev` and `diagrams` services |
| `.githooks/` | The pre-commit hook, which runs `make check` |
| `docs/` | The engineering documents, listed with their status in `docs/index.md` |
| `docs/diagrams/` | PlantUML sources, the shared `style.puml`, and the rendered SVG of each |
| `scripts/` | The checks that `make` runs, and the generator of the Unicode tables |
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
so a change that passes `make check` locally passes the documentation jobs.
The workflow files are added with the first crate, planned for 0.1, with
these jobs in this order: static checks, the test matrix of the test plan,
the package check, and the documentation build with the diagram
comparison. Workflows hold read-only permissions unless a job needs more.

## 9. Releases

1. A commit titled "Prepare X.Y.Z" sets the version and dates the
   changelog's section for it.
2. A tag `vX.Y.Z` on that commit starts the release workflow.
3. The workflow checks that the tag matches the version and the changelog,
   runs every test level of the test plan, and builds the release.
4. A maintainer approves the publishing step.
5. The workflow publishes with trusted publishing and no long-lived token,
   installs the published version back, and creates the release from the
   changelog section.

A release that breaks compatibility goes out as a release candidate first.
The release workflow is added before 0.1 ships.
