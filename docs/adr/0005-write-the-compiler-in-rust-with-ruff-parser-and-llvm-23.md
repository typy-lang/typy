# ADR-0005: Write the compiler in Rust, with Ruff's parser and LLVM 23

## Status

Accepted

## Date

2026-11-05

## Context

The compiler needs a Python parser, an LLVM binding and an implementation
language. The specification pack proposed a first compiler in Python with
llvmlite (its ADR-009), and a later one in Rust or in TyPy itself.

The requirements set targets the compiler has to meet: a debug build of a
10,000-line project in under 2 seconds and a release build in under 20
(NFR-04, NFR-05), at most 1 GB of memory (NFR-14), byte-identical output
(NFR-06), and no crash on any input (NFR-09).

Under ADR-0002, TyPy's grammar is Python's, so the parser has to follow
Python's grammar as it changes. The research compared the options on
2026-11-05 (research sections 5.1 to 5.3):

| Component | Version | Notes |
| --- | --- | --- |
| `ruff_python_parser` | 0.0.20, 2026-10-27 | Covers Python syntax up to 3.15; internal to Ruff, released about weekly |
| `rustpython-parser` | 0.4.0, 2024-08-06 | No release in 27 months |
| `inkwell` | 0.11.0, 2026-10-20 | Supports LLVM 19 to 23 by feature flag |
| LLVM | 23.1.5, 2026-11-03 | Debian trixie ships 19; apt.llvm.org ships 19 to 23 |

Experiment X1 parsed a sample written in standard Python with
`ruff_python_parser` with no errors, and rejected the pack's own syntax with
several errors reported in one pass.

## Options considered

1. **Python with the `ast` module and llvmlite.** The fastest start, and
   `ast` always matches the running CPython. But the build-time targets are
   hard to meet in Python, and the pack already expected a rewrite.
2. **Rust with a parser of TyPy's own.** Full control over error recovery
   and spans. But every change to Python's grammar has to be followed by
   hand, which buys nothing under ADR-0002.
3. **Rust with `rustpython-parser`.** A published, stable crate. But it has
   not been released for 27 months and does not follow current Python
   syntax.
4. **Rust with `ruff_python_parser` and `inkwell`.** A parser that follows
   new Python syntax quickly, with source spans and error recovery, used by
   Ruff and Astral's type checker. Its cost is that the crate makes no
   stability promise.
5. **C++ against LLVM's own API.** No binding layer. But C++ gives no memory
   safety to the compiler itself, which makes NFR-09 harder to meet.

## Decision

TyPy uses option 4.

- The compiler is written in Rust (C-01), built with the toolchain pinned in
  `rust-toolchain.toml`, Rust 1.99.1 at the time of this decision.
- Python source is parsed with `ruff_python_parser`, pinned to one exact
  version. It is upgraded at most once per TyPy minor release, after the
  full conformance suite passes on the new version (C-02). The parser is
  used only behind one module of the compiler, so its API changes stay in
  that module.
- LLVM is used through `inkwell`, with one LLVM major version per TyPy
  release (C-03). TyPy 0.1 uses LLVM 23, installed from apt.llvm.org.

## Consequences

- The compiler can meet the build-time targets without a rewrite.
- New Python syntax reaches TyPy through a parser upgrade, not through new
  parser code.
- Ruff's parser API can change in any release, so each upgrade may need
  work in the one module that wraps it. The conformance suite decides
  whether an upgrade is accepted.
- The devcontainer and CI install `llvm-23-dev` from apt.llvm.org, because
  Debian trixie provides only LLVM 19. The LLVM major version becomes part
  of each TyPy release's identity.
- `inkwell` links LLVM statically or dynamically depending on how LLVM is
  installed, and this choice shall be fixed in the build so NFR-06 holds.
- The compiler and its runtime use dependencies under MIT and Apache-2.0
  (C-04).
- The pack's ADR-009 (a first compiler in Python) does not apply to this
  project.

## Related requirements

C-01, C-02, C-03, C-04, NFR-04, NFR-05, NFR-06, NFR-09, NFR-14, OI-02,
OI-03.
