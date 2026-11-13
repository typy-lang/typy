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
| [ADR-0009](0009-hold-collections-in-the-runtime-and-measure-what-programs-link.md) | Hold the built-in collections in the runtime, and measure the runtime a program links | Accepted | 2026-11-08 |
| [ADR-0010](0010-hash-and-order-sets-and-dicts-as-cpython-does.md) | Hash and order sets and dicts as CPython does | Accepted | 2026-11-09 |
| [ADR-0011](0011-hold-none-in-a-value-each-type-never-takes.md) | Hold None in a value each type never takes | Accepted | 2026-11-09 |
| [ADR-0012](0012-root-every-type-at-object-with-a-type-object-for-each.md) | Root every type at object, with a type object for each | Accepted | 2026-11-10 |
| [ADR-0013](0013-make-exceptions-objects-of-exception-classes.md) | Make exceptions objects of exception classes | Accepted | 2026-11-10 |
| [ADR-0014](0014-iterate-with-runtime-iterator-objects-and-the-iterator-protocol.md) | Iterate with runtime iterator objects and the iterator protocol | Accepted | 2026-11-11 |
| [ADR-0015](0015-compile-modules-into-one-program-with-module-objects.md) | Compile modules into one program, with module objects | Accepted | 2026-11-11 |
| [ADR-0016](0016-compile-each-module-separately-and-verify-interfaces-at-link-time.md) | Compile each module separately and verify interfaces at link time | Proposed | 2026-11-11 |
| [ADR-0017](0017-warn-where-cpython-warns-while-compiling.md) | Warn where CPython warns while compiling | Accepted | 2026-11-12 |
| [ADR-0018](0018-provide-math-and-sys-as-checker-tables-over-runtime-functions.md) | Provide math and sys as checker tables over runtime functions | Accepted | 2026-11-12 |
| [ADR-0019](0019-open-text-files-over-buffered-descriptors-with-cpythons-errors.md) | Open text files over buffered descriptors with CPython's errors | Accepted | 2026-11-12 |
| [ADR-0020](0020-bound-nesting-as-cpython-does-and-compile-on-a-large-stack.md) | Bound nesting as CPython does and compile on a large stack | Accepted | 2026-11-13 |
