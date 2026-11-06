# ADR-0007: Write standard library modules in Python, compiled by TyPy

## Status

Accepted

## Date

2026-11-06

## Context

TyPy provides its own implementation of each standard library module it
supports, with the names and results the Python library documentation
defines (LIB-09, ADR-0003). The list grows by release: `sys` and `math` in
0.3, `dataclasses` and `typing` in 0.4, `os`, `time` and `random` by 1.0
(LIB-01 to LIB-07).

Some of a module's behaviour needs the operating system or raw memory:
writing to a file descriptor, reading the command line, calling `libm`. Most
of it is ordinary logic: `math.gcd`, `os.path.join`, `random.shuffle`,
`dataclasses.field`.

## Options considered

1. **Every module in the runtime, in Rust.** One place for all of it. But
   the runtime grows with every module, against NFR-02, and Rust code cannot
   be checked against CPython by the conformance suite in the way a Python
   module can.
2. **Every module in Python, compiled by TyPy, with a private module of
   primitives.** Each module is ordinary typed Python that also runs under
   CPython, so its behaviour can be compared with CPython's own module. The
   few operations Python code cannot express come from a private module
   that only the standard library can import.
3. **Use CPython's own pure Python modules.** Little work. But they rely on
   dynamic features TyPy rejects, on C extension modules, and on untyped
   code, so most of them would not compile.

## Decision

TyPy uses option 2.

- Each supported module is a typed Python source file in the crate
  `typy-stdlib`, written within the TyPy subset and embedded in the `typy`
  binary when it is built. `typy` reads no standard library file from disk.
- Operations that need the operating system or raw memory are declared in a
  private module, `_typy`, and implemented in the runtime (ADR-0006).
- Only modules inside `typy-stdlib` may import `_typy`. An import of `_typy`
  from a user program is rejected like any module TyPy does not provide
  (LIB-08), so STD-07 holds for every user program.

## Consequences

- The standard library is tested the same way as user programs: each module
  has conformance tests that compare it with CPython's module of the same
  name.
- The modules exercise the compiler on real code, which finds compiler
  defects early.
- A module costs nothing in the runtime until a program imports it, and the
  runtime grows only by the primitives in `_typy`.
- A standard library module can use only the features of the release it
  ships in. A module that needs a later feature, such as generators before
  1.0, waits for that release or is written around it.
- Embedding the modules in the binary keeps `typy` a single file and keeps
  the build deterministic (NFR-06).

## Related requirements

LIB-01 to LIB-09, STD-07, NFR-02, NFR-06, NFR-10, ADR-0003, ADR-0006.
