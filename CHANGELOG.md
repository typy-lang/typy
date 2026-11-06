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
- A quality gate, `make check`, that lints the documents, checks for
  non-ASCII text, checks that every requirement has a row in the test plan,
  and checks that every diagram is rendered from its source.
