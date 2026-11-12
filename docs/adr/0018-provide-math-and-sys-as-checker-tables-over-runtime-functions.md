# ADR-0018: Provide math and sys as checker tables over runtime functions

## Status

Accepted

## Date

2026-11-12

## Context

ADR-0007 decides that each standard library module is a typed Python file
in `typy-stdlib`, with the operations Python cannot express in a private
module, `_typy`, and allows a module that needs a later feature to be
"written around it". The builtins already are: `print`, `sorted` and
`sum` are a table in the checker and runtime functions (detailed design
section 6).

`math` and `sys` are the first standard library modules (LIB-04, LIB-05).
Their functions cannot be declared in the 0.3 subset:

- `math.sqrt(4)` takes an `int` or a `float`. TyPy refuses an `int` where
  a `float` is declared (TYP-09), and `int | float` unions arrive in 0.4.
- `math.floor(x)` returns the `int` itself for an `int` and rounds a
  `float`, a choice by the argument's type that needs overloads.
- `math.gcd(*integers)`, `math.hypot(*coordinates)` and `print(..., file=)`
  take any number of arguments or keywords (EXP-17).
- Most of `math` is a call of the C maths library, with CPython's checks
  around it, and `sys` reads the command line and the standard streams,
  all of which would be `_typy` primitives with a Python wrapper of one
  line each.

## Options considered

1. **Python files now, typed `float`.** Follows ADR-0007 to the letter,
   but `math.sqrt(4)` is refused until 0.4, a gap against CPython in the
   most common uses of the module.
2. **Checker tables over runtime functions, as the builtins.** Each name
   of a module is an entry in a table the checker types; lowering calls a
   runtime function that holds CPython's algorithm and checks. Arguments
   take the types CPython accepts.
3. **Wait for 0.4.** Leaves LIB-04 and LIB-05 unmet in the release that
   plans them.

## Decision

TyPy uses option 2 for `math` and `sys` in 0.3, as the case ADR-0007
allows for a module that needs a later feature.

- A module is a built-in module: `import math` binds a module object that
  prints as CPython's does, `<module 'math' (built-in)>`, and runs no
  code. Its names are entries of a table in `typy-check`, each with its
  shape: the types of its arguments, its result, and the Python version
  that added it.
- A name added after the program's Python floor is E0425 at that floor,
  as CPython of that version has no such name (STD-01): `math.fma` needs
  3.13.
- A function takes the arguments CPython accepts: a real-number
  parameter takes an `int`, a `bool` or a `float`, converted as CPython
  converts it, so an `int` too large for a `float` raises CPython's
  `OverflowError`. An argument CPython refuses for every value, such as a
  `str`, is E0404 with CPython's message.
- The runtime functions port CPython's own code where CPython does not
  call the C library directly: the checks of `math_1` and `math_2`, `pow`,
  `fmod`, `remainder`, `hypot`, `log` of a large `int`, `ldexp`, `frexp`,
  `modf`, `isclose`, `nextafter` and `ulp`, and the integer functions.
  Messages are CPython 3.15's, the newest wording (OI-14).
- A module moves to a Python file in `typy-stdlib` when the features it
  needs arrive, with the same names and results.

## Consequences

- `math` and `sys` work in 0.3 with CPython's results, and conformance
  tests compare them with CPython's modules, as ADR-0007 asks.
- The checker carries one table per built-in module, and the runtime the
  functions behind it; neither grows a program that does not import the
  module, since an unused runtime function is not linked (NFR-01).
- On the `musl` target the C library's results may differ from glibc's
  in the last bit for some functions; conformance tests run on glibc.
- `fsum`, `prod`, `sumprod`, `dist`, `gamma` and `lgamma` are planned for
  0.4 (LIB-12): the first four take iterables of numbers, which need
  generics, and the last two are CPython's own algorithms rather than
  calls of the C library.

## Related requirements

LIB-04, LIB-05, LIB-09, LIB-12, TYP-09, EXP-17, STD-01, OI-14, ADR-0003,
ADR-0007.
