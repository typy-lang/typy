# ADR-0029: Report CPython 3.15 as the Python version of a built program

## Status

Accepted

## Date

2026-11-14

## Context

LIB-13 asks for `sys.version`, `sys.version_info` and `sys.hexversion`.
In CPython they give the version of the interpreter that runs the
program. A program TyPy builds runs without an interpreter, so what they
report is a choice. Programs read them for two reasons: to print the
version, and to check it, as in `if sys.version_info >= (3, 12):`.

TyPy follows the behaviour of CPython 3.15, with its results and its
messages (OI-14), whatever the program's floor. The floor,
`python-floor` in `typy.toml`, only decides which names a program may
use (LIB-05).

`sys.version_info` in CPython is a struct sequence: it prints as
`sys.version_info(major=3, minor=15, micro=0, releaselevel='final',
serial=0)`, has the fields `major` to `serial`, and indexes and compares
as a tuple. TyPy compares tuples of the same length and item types only,
and has no struct sequence type.

## Options considered

1. **Report CPython 3.15.0.** The version whose behaviour the program
   has, so a version check answers as it would on that interpreter.
2. **Report the program's floor.** The version the program was written
   against; it under-reports, since the program's messages and results
   are 3.15's.
3. **Refuse the three names** until a struct sequence type exists.

## Decision

TyPy uses option 1.

- `sys.version` is `'3.15.0 (TyPy X.Y.Z)'`, with the version of the
  compiler, so `sys.version.split()[0]` is `'3.15.0'`.
- `sys.hexversion` is `0x030F00F0`.
- `sys.version_info` is folded where a program reads it through `sys`:
  a comparison with a tuple of `int` constants, an item at a constant
  index, and the fields `major`, `minor`, `micro`, `releaselevel` and
  `serial` become constants of `(3, 15, 0, 'final', 0)`. Any other use,
  as a value that is printed, stored or imported with
  `from sys import version_info`, is E0201 until LIB-16.

## Consequences

- A version check costs nothing at run time and needs no value of a new
  type.
- When TyPy follows a newer CPython, the constants change with it, in
  the release that moves.
- A comparison that CPython would refuse with `TypeError`, such as
  `sys.version_info >= (3, 15, 0, 0)`, which compares `'final'` with an
  `int`, is not folded, so it is refused when the program is compiled.

## Related requirements

LIB-05, LIB-13, LIB-16, OI-14, ADR-0018.
