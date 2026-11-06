# ADR-0003: Implement Python independently, with no names of its own

## Status

Accepted

## Date

2026-11-05

## Context

Under ADR-0002, a TyPy program is a Python program that CPython can run. The
specification pack also needed concepts that Python does not have: fixed
width integers such as `i32`, raw pointers, `unsafe` blocks, and
declarations of C functions and exports.

The research tried one way to express them in standard Python: a `typy`
package imported by the program (research experiment X2). The program ran
under CPython, but only with the package installed, and the package then
defined what those names meant under CPython. The PyPI name `typy` is also
taken by an unrelated project.

The project owner decided that TyPy is an independent implementation of
Python, in the way PyPy is, and not a module, package or library added to
Python.

## Options considered

1. **A `typy` package of TyPy-specific names**, installed into CPython for
   testing. It gives direct control over C types and overflow. But every
   program that uses it depends on a TyPy artefact to run under CPython, the
   oracle is no longer plain CPython, and the package name clashes on PyPI.
2. **TyPy-specific hints in comments or decorators from a private module.**
   The source stays importable. But hints in comments are invisible to type
   checkers, and a private module is option 1 under another name.
3. **An independent implementation that adds no names.** Programs use only
   the Python language and the standard library. TyPy supplies its own
   implementation of each standard library module it supports. C calls use
   the standard `ctypes` module, and build settings live in `typy.toml`.

## Decision

TyPy uses option 3.

- TyPy adds no module, function, type, decorator or syntax of its own
  (STD-07).
- Each standard library module TyPy supports is TyPy's own implementation,
  with the names and results the Python library documentation defines
  (LIB-09). Importing a module TyPy does not provide is rejected with the
  list of those it does (LIB-08).
- C functions are declared through `ctypes`, with a literal library name and
  `argtypes` and `restype` assigned at module level. TyPy compiles them to
  direct calls and links the library at build time (FFI-01 to FFI-03).
- Static or dynamic linking, and the functions exported to C with their C
  types, are set in `typy.toml` (FFI-04, FFI-05).
- There are no fixed-width integer types. `int` has no size limit, and small
  values are held inline (DAT-01).

## Consequences

- The oracle is plain CPython with nothing installed, so any developer can
  check TyPy's output with a stock interpreter.
- A program that calls C runs under CPython through `ctypes` and under TyPy
  through a direct call, with the same conversions and wrap-around.
- TyPy carries the cost of its own standard library: each supported module
  is written, documented and tested against CPython's behaviour. The
  supported list grows release by release (LIB-01 to LIB-07).
- Code that wants machine integers without an overflow check has no way to
  ask for them. Performance relies on the inline `int` (ADR-0001) and on the
  compiler proving ranges.
- The code that NFR-07 does not cover is marked by its `ctypes` calls, not
  by an `unsafe` block.
- Nothing is published to PyPI under the `typy` name, so the clash does not
  arise.

## Related requirements

STD-07, LIB-01 to LIB-09, FFI-01 to FFI-07, DAT-01, NFR-07, A-02, OI-04,
OI-05, OI-06.
