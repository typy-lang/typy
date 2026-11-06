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
- A quality gate, `make check`, that checks formatting, runs Clippy and the
  tests with at least 95 percent line coverage, lints the documents, checks for
  non-ASCII text, checks that every requirement has a row in the test plan,
  and checks that every diagram is rendered from its source.
