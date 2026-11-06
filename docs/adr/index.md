# Architecture decision records

Each record states one decision, with the options considered and the
consequences. An accepted record is not rewritten; a later record
supersedes it.

| ADR | Decision | Status | Date |
| --- | --- | --- | --- |
| [ADR-0001](0001-use-optimised-reference-counting-with-cycle-collection.md) | Use optimised reference counting with selective cycle collection | Accepted | 2026-11-05 |
| [ADR-0002](0002-accept-only-standard-python-with-cpython-meaning.md) | Accept only standard Python, with the meaning CPython gives it | Superseded by ADR-0008 | 2026-11-05 |
| [ADR-0003](0003-implement-python-independently-with-no-names-of-its-own.md) | Implement Python independently, with no names of its own | Accepted | 2026-11-05 |
| [ADR-0004](0004-compile-exceptions-as-error-values-returned-to-the-caller.md) | Compile exceptions as error values returned to the caller | Accepted | 2026-11-05 |
| [ADR-0005](0005-write-the-compiler-in-rust-with-ruff-parser-and-llvm-23.md) | Write the compiler in Rust, with Ruff's parser and LLVM 23 | Accepted | 2026-11-05 |
| [ADR-0006](0006-write-the-runtime-in-rust-without-the-standard-library.md) | Write the runtime in Rust without the standard library | Accepted | 2026-11-06 |
| [ADR-0007](0007-write-standard-library-modules-in-python-compiled-by-typy.md) | Write standard library modules in Python, compiled by TyPy | Accepted | 2026-11-06 |
| [ADR-0008](0008-judge-each-program-from-its-own-python-floor.md) | Judge each program by CPython from its own Python floor | Accepted | 2026-11-06 |
