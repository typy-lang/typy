# TyPy development guide

| Field | Value |
| --- | --- |
| Document | TyPy development guide |
| Version | 0.29 |
| Status | Draft |
| Owner | typy-lang contributors |
| Last updated | 2026-11-22 |

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
TyPy's build of LLVM, the cargo tools and Node, and starts the `dev`
service. `make check` runs
the gate once to show the set-up works.

To work in VS Code, open the folder and run "Dev Containers: Reopen in
Container". VS Code attaches to the same `dev` service.

### 2.1 Working on a Mac

macOS on Apple silicon builds for itself and runs natively, outside the
container (ADR-0055). The Mac needs Xcode's Command Line Tools, which
provide `cc`, Apple's linker, `libtool` and `dsymutil`, and rustup, which
installs the toolchain `rust-toolchain.toml` names. Then:

1. Download `llvm-23.1.6-aarch64-apple-darwin.tar.zst` from the release
   `.devcontainer/llvm.lock` names, check it with `shasum -a 256` against
   the lock, unpack it, and set `LLVM_SYS_231_PREFIX` to the unpacked
   directory.
2. Install the CPython oracles the Dockerfile lists in `ORACLE_PYTHONS`
   with `uv python install`, so `python3.12` to `python3.15` are on
   `PATH`, and `cargo-nextest` at the version of `CARGO_NEXTEST_VERSION`.
3. Run a target without the container: `make EXEC= test`.

The `macos-15` job of CI does the same on every pull request.

## 3. Everyday commands

| Command | What it does |
| --- | --- |
| `make dev` | Starts the `dev` service, building its image the first time |
| `make shell` | Opens a shell in the `dev` service |
| `make build` | Builds the compiler, `<target>/debug/typy`, after `make runtime` |
| `make runtime` | Builds the runtime archives for `x86_64-unknown-linux-gnu` and `x86_64-unknown-linux-musl` with `scripts/build-runtime.sh`, one for speed and one for size (ADR-0100), and stages them in `<target>/lib/typy/<triple>/` and `<target>/lib/typy-small/<triple>/`, where a compiler in `<target>/debug` or `<target>/release` finds them; `make test` and `make coverage` run it first |
| `make bless` | Rewrites the golden files in `tests/golden/` from the current compiler; review the diff before committing |
| `make miri` | Runs the runtime's unit tests under Miri with the pinned nightly toolchain, in about 8 minutes |
| `make fuzz` | Fuzzes the source, checking and lowering stages for `FUZZ_SECONDS` seconds, 60 by default; a crash leaves its input in `fuzz/artifacts/` |
| `make runtime-size` | Links a hello world for each target and fails when the runtime it links is over the size budgets of NFR-02 |
| `make mutants` | Mutation-tests the Rust lines changed since `MUTANTS_BASE`, by default the work not yet committed, new files included, each mutant against its own crate's tests; a slice takes minutes. It is run on demand, for a change whose tests deserve a closer look, and each survivor is then caught by a new test, removed as dead code, or listed in `.cargo/mutants.toml` with the reason it cannot change a result. It runs in a container limited to 6 GB of memory with no swap, 4 CPUs and no network, made from an image of the dev container that is removed afterwards; the survivors are in `.tmp/mutants/diff/mutants.out/missed.txt` |
| `make mutants-package MUTANTS_PACKAGE=typy-check` | Mutation-tests every mutant of one crate, which takes hours for a large one; for a review of a crate's tests, not for each change |
| `make bench-compare` | Builds the release compiler and runs `tests/bench/compare.py`: each workload of `tests/bench/programs/` built by TyPy and by every peer of `make bench-peers`, its output checked against CPython 3.15's, then timed three times pinned to two cores with its peak memory; `tests/bench/report.py` writes the tables of the [benchmark report](../testing/benchmark-report.md). It takes about 90 minutes and is run on demand, on a quiet machine; `BENCH_WORKLOADS="nbody dicts"` runs a subset. `BENCH_LANES=3` runs three jobs at once, each pinned to a physical core of its own other than core 0, for a quicker check whose times run higher and vary more, as the lanes share the caches and a busy chip lowers its clock; the report names the cores and the jobs at once, and a published report runs one |
| `make bench` | Builds the compiler in release mode and times it on generated 10,000-line projects, in one file and in 200 modules, against the budgets of NFR-04, NFR-05 and NFR-14, with a check that checking time grows linearly with the modules |
| `make bench-peers` | Downloads, once, the languages the cross-language benchmark compares with TyPy that the image lacks, about 0.5 GB: Go 1.27.4, the Temurin JDK 25.0.5, PyPy 3.11 v7.3.23 and Codon 0.20.6 from their release archives, and mypyc, Cython and Nuitka into a CPython 3.14 environment from `tests/bench/peers/requirements.txt`. Each archive is checked against the sha256 its project publishes and each wheel against its hash; everything goes into the `bench-tools` volume, `/var/cache/typy/bench`, and a tool already there is kept. It also links `lib/libz.so` there to the image's zlib, which Codon links its programs with |
| `make stdlib-names` | Writes `crates/typy-source/src/stdlib_names.rs`, the standard library module names of CPython 3.12 to 3.15, which the loader leaves to the checker (LIB-08); `make check` runs `make stdlib-names-check` |
| `make check` | Runs the sweep below, then the whole quality gate: formatting, Clippy, the tests with line and branch coverage, the five documentation checks below, the Unicode table check and the runtime size check |
| `make fmt` | Formats the Rust code |
| `make lint` | Runs Clippy on every crate and test, with warnings as errors |
| `make type` | Type checks every crate and test |
| `make test` | Runs the unit tests with `cargo nextest` |
| `make coverage` | Runs the tests with coverage on the stable toolchain and fails below 96 percent of lines |
| `make coverage-branches` | Runs the tests with coverage on the pinned nightly toolchain and fails below 96 percent of lines or of branches (NFR-11); `make check` runs it |
| `make test-int` | Runs the integration tests: the conformance suite and its sanitizer run, the golden files and the pipeline |
| `make test-e2e` | Runs the end-to-end tests of the `typy` executable |
| `make test-all` | Runs `make test` and `make miri` |
| `make sweep` | Deletes the build outputs Cargo keeps but no build uses: in each `deps` directory of the build volume, every copy of a crate's library or a test program but the newest and those rebuilt in the last 12 hours, and incremental caches untouched for 12 hours. A test program that links LLVM is about 200 MB, and Cargo keeps a copy for every change of code, version or flags, so without the sweep the volume grows past 30 GB |
| `make docs-lint` | Lints every Markdown file with markdownlint-cli2 |
| `make docs` | Builds the documentation site into `site/` with `scripts/build-docs.sh`: Zensical in strict mode, which fails on a link to a page or an anchor that does not exist (DOC-05). The first run installs Zensical 0.1.1 and its dependencies from `scripts/docs-requirements.txt`, each checked against its sha256, into a CPython 3.14 environment at `/var/cache/typy/docs` in the dev container, about 20 MB to download, which a container built anew installs again. Open `site/index.html`, or serve the directory, to read the result |
| `make ascii-check` | Fails on any non-ASCII byte in tracked text files, except rendered SVG |
| `make traceability` | Fails when a requirement has no row in the test plan's matrix, a document cites an ID that is not defined, or a diagnostic plans a requirement for another release than the SRS gives it |
| `make diagrams` | Renders every PlantUML source to the SVG beside it |
| `make diagrams-check` | Renders, then fails when an SVG differs from the one staged |
| `make unicode-tables` | Writes `crates/typy-runtime/src/string/unicode/tables.rs` from the `unicodedata` of CPython 3.15 (OI-15); never edit that file by hand |
| `make unicode-tables-check` | Fails when the committed Unicode tables differ from the ones CPython 3.15 gives |
| `make format-cases` | Writes `crates/typy-runtime/src/format/tests/cases.rs`, the results `format(value, spec)` gives under CPython 3.12 to 3.15 for a table of cases, keeping those all four agree on; never edit that file by hand |
| `make format-cases-check` | Fails when the committed table of `format()` results differs from the one CPython 3.12 to 3.15 give |
| `make image` | Rebuilds the dev image after a change under `.devcontainer/` |
| `make clean` | Stops the containers and removes `.tmp/preview`, `dist/`, `site/` and Zensical's cache, `.cache/` |

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
NFR-14; CI runs it after the release archive. The user documentation, every
page under `docs/` outside the engineering documents' directories, is
Markdown that `make docs-lint` checks, and
`crates/typy-conformance/tests/integration/documentation.rs` judges its
samples (DOC-04).

## 4. Repository layout

| Path | Holds |
| --- | --- |
| `.devcontainer/` | The dev image, and the compose file with the `dev` and `diagrams` services |
| `.github/` | The CI, release and code scanning workflows, Dependabot's settings and the pull request template |
| `.githooks/` | The pre-commit hook, which runs `make check` |
| `docs/` | The user documentation, whose sections `docs/index.md` lists: `tutorial/`, `reference/`, `library/`, `using/`, `extending/` and `diagnostics/`; and the engineering documents, listed with their status in `docs/engineering.md` |
| `docs/diagrams/` | PlantUML sources, the shared `style.puml`, and the rendered SVG of each |
| `fuzz/` | The fuzz targets, a Cargo workspace of their own that builds with the pinned nightly |
| `scripts/` | The checks and builds that `make` runs, the release scripts, the build of LLVM, and the generator of the Unicode tables |
| `tests/conformance/` | The conformance programs, by requirement area |
| `tests/golden/` | The golden programs and the expected text of each stage and diagnostic |
| `dist/` | Release archives that `make dist` builds. Ignored by Git |
| `mkdocs.yml` | The settings and the navigation of the documentation site |
| `site/` | The documentation site that `make docs` builds. Ignored by Git |
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
  `NNNN-the-decision-in-words.md`, a row in `docs/adr/index.md`, and a
  line in the `nav` of `mkdocs.yml`. An accepted record is not rewritten;
  a later record supersedes it.
- A new page gets a line in the `nav` of `mkdocs.yml`, where the site
  shows it; `make docs` fails on a page the navigation leaves out, through
  `scripts/check-docs-nav.py` (DOC-05). A link to a file outside `docs/`,
  such as a source file, is a link to it on GitHub,
  `https://github.com/typy-lang/typy/blob/main/` and its path, since the
  site holds only `docs/`.
- A document's header carries its version, status and date. A change
  raises the version and the date.

### 5.1 Adding a diagram

1. Write `docs/diagrams/<type>-<subject>.puml`, starting with
   `!include style.puml` and a `title`.
2. Run `make diagrams` and open the SVG. Check that nothing overlaps and
   that it reads at the width of the site; split a diagram that does not.
3. Add a row for it to the diagram table in `docs/engineering.md`.
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
read-only permissions; only the jobs that create a release may write to the
repository, and only the job that deploys the documentation site may write
to GitHub Pages.

| Workflow | Job | Runs on | Does |
| --- | --- | --- | --- |
| `ci.yml` | Quality gate and package | Every pull request | `make check`, then `make dist`; for a pull request of documents alone, the light run below |
| `ci.yml` | Miri and fuzzing | On demand, before a release | `make miri`, then `FUZZ_SECONDS=1800 make fuzz`; the inputs that crashed are kept as an artifact |
| `ci.yml` | AArch64 tests | Every pull request but one of documents alone, on an `ubuntu-24.04-arm` runner | `make test`: every unit, integration and conformance test natively, and the conformance suite built for x86-64 under `qemu-x86_64` (NFR-16), then `make dist`; built with line tables for debug information, since the runner has 14 GB of disk |
| `ci.yml` | macOS tests | Every pull request but one of documents alone, on a `macos-15` runner | `make EXEC= test` natively, with the LLVM `llvm.lock` pins from the local action `pinned-llvm` and the CPython oracles installed with uv (NFR-17, ADR-0055), then `make EXEC= dist`; the sanitizer run and the other architecture under qemu stay on Linux |
| `ci.yml` | Dependency review | Every pull request | Fails on a new dependency with a known vulnerability |
| `image-cache.yml` | Dev image | A push to `main` that changes `.devcontainer/`, `rust-toolchain.toml` or the dev image action, for x86-64 and arm64; on demand | Builds the dev image into GitHub's Actions cache, which the jobs below read |
| `codeql.yml` | Analyze | Every push to `main` that changes more than documents, and weekly | Code scanning of the Rust sources and the workflows, leaving out the test files that `.github/codeql/codeql-config.yml` lists |
| `release.yml` | Check and build, Build for AArch64 hosts, Build for macOS, Publish, then Documentation | A tag `vX.Y.Z` | Section 9 |
| `llvm.yml` | Build for each host, then Publish | On demand, once per LLVM version | Section 10 |
| `pages.yml` | Build the site, then Deploy | Called by `release.yml` once a release is created; a release published by hand; on demand, on a release tag | Builds the site with `scripts/build-docs.sh` on the runner after `astral-sh/setup-uv`, without the dev image, and deploys it to GitHub Pages at <https://typy-lang.org> (DOC-05) |

A pull request whose every file is a document that no compiled test reads
gets a light run of the same job, which builds no dev image and takes a
minute or two: markdownlint, the strict site build with its navigation
check, the diagrams, the ASCII check and traceability. `scripts/ci-changes.sh`
decides from the files the pull request changes. The engineering documents,
the diagrams, the site's settings and style, and the Markdown files at the
root and in `.github/` qualify; so does a page of the user documentation
while it holds no Python block, which the documentation test compiles
(DOC-04). The troubleshooting page, which `typy explain` embeds, and every
other file give the full run.

Every job that needs the dev container starts with the local action
`.github/actions/dev-image`. It builds the image with Docker's buildx from
the layers GitHub's Actions cache holds for the runner's architecture, which
`image-cache.yml` fills on `main`, and loads it under the name `compose.yaml`
gives, so `make dev` builds nothing; a job loads it in about a minute, where
building it took five or more. A pull request reads `main`'s cache and keeps
the layers it builds for its own later runs. The workflow's short-lived
token reaches the build as a secret, so cargo-binstall asks GitHub's API for
its binaries without reaching the limit of a shared runner, and is kept in
no layer. The x86-64 job deletes unused preinstalled toolchains in the
background while the image loads, and waits for the room before the quality
gate.

CI minutes are spent on whole changes. `main` changes only through a pull
request that is up to date and passes the quality gate, so CI does not run
again after the merge, and a change is pushed when it is complete, never as a
series of partial pushes. Dependabot proposes one pull request a month for
the actions and one for the Cargo dependencies of the workspace and of
`fuzz/`. The repository's settings protect
`main`: pull requests only, the quality gate job required, linear history,
squash merges and no force pushes; and the `release` environment requires a
maintainer's approval. GitHub Pages is built by GitHub Actions, with the
custom domain `typy-lang.org` and HTTPS enforced, and the `github-pages`
environment lets the tags `v*` deploy, since the site is built from a
release's tag. A merged branch is deleted, except a release branch:
the ruleset "release branches" forbids deleting any `release-*` branch, so
each release's branch stays on origin and an issue can be traced to it.

## 9. Releases

1. `make miri` and `FUZZ_SECONDS=1800 make fuzz` pass, locally or through
   a run of `ci.yml` started by hand (NFR-07, NFR-09).
2. A commit titled "Prepare X.Y.Z" sets the version in `Cargo.toml`,
   renames the changelog's "Unreleased" section to `[X.Y.Z] - DATE`, and
   opens a new empty "Unreleased" section above it. For a new minor
   release it adds the release to `RELEASES` in
   `crates/typy-conformance/tests/integration/suite.rs`, so the suite requires an
   accepted and a rejected test of every requirement up to it. It also
   sets the version of the documentation, in `site_name` in `mkdocs.yml`
   and the title of `docs/index.md`, and adds `docs/whatsnew/X.Y.md`, the
   release's What's new page, which `docs/whatsnew/index.md`, the start
   page and the `nav` of `mkdocs.yml` link.
3. A tag `vX.Y.Z` on that commit starts `release.yml`.
4. `scripts/check-release.sh` checks that the tag names the version and that
   the changelog has a dated section for it, whose text becomes the release
   notes. The job then runs `make dist` for x86-64 hosts, while a job on an
   `ubuntu-24.04-arm` runner runs it for AArch64 Linux hosts and one on a
   `macos-15` runner runs `make EXEC= dist` for Macs. None runs the quality
   gate again: `main` changes only through pull requests whose latest
   commit passed it, so the tag's tree has.
5. A maintainer approves the `release` environment.
6. The publish job attests each archive's build provenance with
   `actions/attest-build-provenance`, which signs it through Sigstore with
   the workflow's OIDC identity, and checks each with `gh attestation
   verify`, so no signing key is kept (NFR-26). It then creates the GitHub
   release with the three archives, their SHA-256 and their bills of
   materials, using the workflow's short-lived token and no long-lived one.
   It downloads them back, checks each SHA-256 and the x86-64 archive's
   attestation, unpacks that archive and runs a program with it; the
   AArch64 and macOS archives ran one on their own runners when they were
   built.
7. Publishing the release deploys its documentation: the documentation job
   calls `pages.yml`, which builds the site from the tag in strict mode and
   deploys it to <https://typy-lang.org> (DOC-05). A run of `pages.yml`
   started by hand on the tag deploys it again.

`make dist` builds `dist/typy-X.Y.Z-HOST.tar.gz` for the host's triple,
`x86_64-unknown-linux-gnu`, `aarch64-unknown-linux-gnu` or
`aarch64-apple-darwin`: the `typy` executable in `bin/` and the runtime archives
in `lib/typy/<triple>/` and `lib/typy-small/<triple>/` for each target the host
builds for, which `scripts/host-targets.sh` lists, in the layout of an
installation prefix, with the licences, the third-party notices, README and
changelog. Its `licenses/` directory holds the licences of the crates `typy`
links, which cargo-about lists with `about.toml` and `about.hbs`, of LLVM and of
Rust's standard library; a crate under a licence `about.toml` does not accept
fails the build. The tarball is reproducible, so two builds of one commit have
the same SHA-256; on a Mac, GNU tar's `gtar` writes it. The executable links
LLVM statically and, on Linux, needs glibc 2.39 or later and `libstdc++`, and on
macOS 11 or later only `libSystem`, `libc++` and `libiconv`, which
`scripts/check-executable-needs.sh` checks before the archive is packed, and a C
toolchain to link programs. LLVM's licence comes from its archive.

`scripts/write-sbom.py` then writes the archive's bill of materials,
`sbom.cdx.json`, in CycloneDX 1.6: every file of the archive with its
SHA-256, every crate the compiler and the runtime link with its licence,
from `cargo tree`, LLVM and the Rust standard library, dated by the commit
so the bill is reproducible (NFR-25). Once the archive is packed,
`scripts/check-sbom.py` fails when the bill misses a file, a crate or a
library, and a copy of the bill goes beside the archive as
`typy-X.Y.Z-HOST.cdx.json`.

A release that breaks compatibility goes out as a release candidate first,
with a version such as `1.0.0-rc.1`, which the workflow marks as a
pre-release.

## 10. Building a new LLVM

TyPy links an LLVM it builds itself from LLVM's signed sources, one build
for each host TyPy runs on (ADR-0054). A build takes one to two and a half
hours, so it runs once per LLVM version, never in a pull request.

1. A maintainer runs `llvm.yml` on `main` from the Actions tab, or with
   `gh workflow run llvm.yml -f version=23.1.6`.
2. Each build job downloads `llvm-project-VERSION.src.tar.xz` from LLVM's
   release `llvmorg-VERSION` and checks its attestation with
   `gh attestation verify`, which fails unless LLVM's own release workflow
   made the archive. `scripts/build-llvm.sh` then builds it: on
   `ubuntu-24.04` and `ubuntu-24.04-arm` inside `debian:trixie-slim`, the
   Debian 13 base of the dev image, so the libraries need no newer glibc
   than the image has; on `macos-15` for macOS 11.0, the oldest macOS on
   Apple silicon.
3. The build is `Release`, with static libraries, the `AArch64` and `X86`
   back ends, no assertions, and no `zlib`, `zstd`, `libxml2`, `z3` or
   `libffi`, so it depends on the host's C and C++ libraries alone. The
   archive `llvm-VERSION-HOST.tar.zst` holds the libraries, the headers,
   `llvm-config` and LLVM's licence.
4. The publish job checks each archive against its SHA-256 and creates the
   pre-release `llvm-VERSION` on the commit the workflow ran from, with
   the three archives, their SHA-256 and release notes that list them. A
   pre-release is never the repository's latest release. The job runs only
   when every build passed, and refuses to replace a release that exists,
   since the archives' bytes are pinned once they are published.
5. A pull request writes the new version, the release's download address
   and each host's SHA-256 from the release notes into
   `.devcontainer/llvm.lock`. The Dockerfile downloads the archive of its
   architecture, checks it against the lock and unpacks it into
   `/opt/llvm`, which `LLVM_SYS_231_PREFIX` names, so every dev image built
   from that commit, a developer's or CI's, links the same LLVM.

To try the build on a machine of your own, put the source archive in an
empty directory and run the script from there:

```sh
scripts/build-llvm.sh 23.1.6 x86_64-unknown-linux-gnu dist
```

It needs CMake, Ninja, Python 3, a C++ compiler, `xz` and `zstd`.
