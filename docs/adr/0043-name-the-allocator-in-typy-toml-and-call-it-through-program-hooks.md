# ADR-0043: Name the allocator in typy.toml and call it through program hooks

## Status

Accepted

## Date

2026-11-17

## Context

MEM-08 asks that a project can replace the allocator through three
functions: allocate, reallocate and free. ADR-0001 names the replaceable
allocator as the base of TyPy's memory management, and ADR-0006 keeps the
runtime free of Rust's standard library, so every object of TyPy 0.4 comes
from `crate::allocator`, which calls the C library's `malloc`, `realloc`
and `free`.

A program that wants another allocator, such as mimalloc, jemalloc or a
pool of its own, has it as a C library. The choice belongs to the project,
not to the source, as the choice of a library's linking does (FFI-04), and
the runtime archive is built once for every program. Three things decide
the design: where the project names the functions, how the runtime's calls
reach them without a rebuilt runtime, and what a program built as an object
file or as a library (CLI-09) carries with it.

## Options considered

1. **Symbols the linker defines.** Each link passes
   `--defsym=typy_rt_heap_allocate=mi_malloc` and the like, and the
   runtime's calls reach the functions with no step between. Every link
   must repeat the definitions: one made from `--emit=obj`, a C program
   that links a TyPy static library, and the sanitizer runs of the test
   suite.
2. **A table of function pointers set at startup.** The runtime reads the
   three functions from a table that the program's entry fills. It is
   process-wide mutable state, and a C program that calls a TyPy library
   would have to fill it before its first call.
3. **Hooks the program defines.** Code generation emits three functions
   with names of the runtime's own in every program object, each calling
   the function the project names, or `malloc`, `realloc` and `free`
   without a setting. The object carries its allocator wherever it is
   linked, at the cost of one jump for each allocation.
4. **Weak default definitions in the runtime.** The project defines
   functions with the runtime's names, and its strong definitions win. The
   project would write C against TyPy's names, and Rust has no stable weak
   linkage.

## Decision

TyPy uses option 3.

- **The setting.** A table `[allocator]` in `typy.toml` holds `allocate`,
  `reallocate` and `free`, each the name of a C function, all three or
  none, and may hold `library`, the name of a library the linker finds as
  it finds `-lname`. A function's name is a C identifier of at most 255
  characters that does not start with `typy_`, `__typy` or `_TY`, the
  prefixes of TyPy's own symbols, so a hook never calls itself or a
  function of the program, and the three names differ, since one function
  cannot have two of the signatures. Each is checked when the file is read,
  and a wrong one is E0107 before any module is read.
- **The functions.** They have the C signatures of `malloc`, `realloc` and
  `free`: `void *allocate(size_t size)`, `void *reallocate(void *pointer,
  size_t size)` and `void free(void *pointer)`. The runtime never asks for
  zero bytes, needs memory aligned as `malloc` aligns it, and ends the
  program with `MemoryError` when one returns null.
- **The hooks.** Every program object defines `typy_rt_heap_allocate`,
  `typy_rt_heap_reallocate` and `typy_rt_heap_free` with hidden visibility,
  so a shared library does not export them. Each calls its function in
  tail position. `crate::allocator` calls the hooks for every allocation
  of the runtime.
- **The link.** The linker adds `-l` and the library's name after the
  runtime archive when the table names a library.

## Consequences

- A project that sets the table uses its allocator for every object,
  with no change to its source, which still runs under CPython.
- An allocation costs one more direct jump than a call of `malloc`, which
  is small beside the allocator's own work.
- An object file from `--emit=obj` links with the runtime archive alone,
  as before.
- The runtime's unit tests define the hooks themselves, over `malloc`,
  `realloc` and `free`, since they link no program object.
- Linking the library statically or dynamically follows the setting that
  FFI-04 adds for every library.

## Related requirements

MEM-01, MEM-08, CLI-07, CLI-09, FFI-04, ADR-0001, ADR-0006.
