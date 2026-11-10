# ADR-0009: Hold the built-in collections in the runtime

## Status

Accepted

## Date

2026-11-08

## Context

Release 0.2 adds `list`, `tuple`, `dict`, `set` and `bytes` (DAT-06 to
DAT-12), slicing, comprehensions, f-strings with format specifications
(LEX-07), and the built-in functions and methods of LIB-02 and LIB-03.
Their algorithms are a growable array, an insertion-ordered hash table, a
stable sort, slice arithmetic, and number formatting with precision and
padding, which has to round as CPython does.

ADR-0001 fixes their layout: a container's items are stored by element
type, so a `list[float]` holds raw doubles and a `list[Point]` pointers
(MEM-02, NFR-13). ADR-0007 puts standard library *modules* in Python, but
these are the built-in types themselves, which the language needs before
any module can be written, and Python code cannot yet be generic over an
element type (GEN, 0.4).

NFR-02, as written for 0.1, capped the runtime archive at 48 KB, and 32 KB
without large integers and cycle collection. In 0.1 the archive was 40.3 KB
and its core 29.8 KB. The 0.2 code is estimated at 30 to 40 KB more. The
archive is not what a user receives: every runtime function is in a section
of its own, and the linker keeps only the sections a program reaches, so a
hello world built with `--release-small` links 10.3 KB of the 40.3 KB.

## Options considered

1. **Generate each container in the compiler, per element type.** The
   runtime stays small, but every algorithm is written a second time as
   code that builds LLVM IR, which is slow to write and to review, and each
   element type used adds a copy to the program.
2. **Write the containers in Python, compiled by TyPy.** It matches
   ADR-0007, but needs generic functions inside the compiler three
   releases before GEN, and number formatting written in TyPy's own subset.
3. **Write them once in the Rust runtime, generic over element layout.**
   One implementation serves every element type. The compiler passes each
   operation an element size and a type descriptor whose functions copy,
   release, compare, hash and format an item. Unused operations are not
   linked. The archive grows past the 0.1 cap.

## Decision

TyPy uses option 3. The built-in collections, `bytes`, slicing, sorting
and formatting are written in the runtime, one module for each Python type
and named as Python names it, generic over the element layout that ADR-0001
fixes. The compiler emits a type descriptor for each element type a
program uses.

NFR-02 measures the runtime a program links, not the archive: the runtime
code and data linked into a hello world built with `--release-small` stays
under 48 KB, and under 32 KB without its large-integer and
cycle-collection parts. `make runtime-size` links the hello world for each
target with a link map and counts the runtime's sections in it.

## Consequences

- Each algorithm exists once, in Rust, with unit tests and Miri, and
  programs carry only the operations they call.
- A program that uses many built-in types links more of the runtime; its
  size is still bounded by NFR-01 for the hello world, and is reported for
  the conformance programs.
- The runtime archive has no cap of its own, so its growth is watched
  through the hello world figure and through review.
- Calls through a type descriptor cost an indirect call per item for
  copying, comparing and hashing; container operations on `int`, `float`
  and `str` items get direct paths where profiling shows the need.

## Related requirements

NFR-01, NFR-02, NFR-13, MEM-02, DAT-06 to DAT-12, LEX-07, LIB-02, LIB-03,
ADR-0001, ADR-0006, ADR-0007.
