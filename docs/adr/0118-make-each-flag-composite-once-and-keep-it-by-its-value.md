# ADR-0118: Make each flag composite once and keep it by its value

## Status

Accepted

## Date

2026-11-23

## Context

A `Flag` or an `IntFlag` combines members with `|`, `&`, `^` and `~`.
CPython's `Flag._missing_` makes a composite for a value no member has,
names it by the members whose bits it holds, such as `R|W`, and keeps it
in the class's `_value2member_map_`, so a second `Perm.R | Perm.W` gives
the same object and `is` holds between them. A value outside the
members' bits raises `ValueError` for a `Flag`, whose text writes the
bits in a two's complement form, and stays for an `IntFlag`, named by its
members and the number of the unknown bits. CPython 3.12 to 3.15 give
the same results for members of one bit each; for a member of several
bits, an alias in CPython's terms, the naming takes other paths.

ADR-0116 writes an enum class as an ordinary class with written methods.
A class body may not hold a class variable of a mutable value, and the
lookup of `Color(value)` goes through a hidden static method.

## Options considered

1. **Make a new composite at each operation,** and compare flags by
   value. `(Perm.R | Perm.W) is (Perm.R | Perm.W)` is `False`, where
   CPython prints `True`.
2. **Keep each composite in a hidden module variable,** a
   `dict[int, Perm]` empty when the class statement runs, filled by a
   written static method that follows `Flag._missing_` for members of
   one bit each.

## Decision

Option 2. A flag's members are each one bit, or `auto()`, which gives the
bit above the highest one before it. The expansion writes the operators,
`__contains__`, `__len__`, `__bool__`, `__repr__` and `__str__`, the
hidden static method `" (flag)"`, which returns the member or the kept
composite of a value or makes one, and the hidden method `" (bits)"`,
which lists the members a flag holds in definition order for a loop and
`in`. The module variable `" (flags) Perm"` follows the class statement.
`Perm(value)` calls `" (flag)"`. An `IntFlag` is also an `IntEnum` in
the sense of ADR-0117, but `|`, `&`, `^` and `~` of an `IntFlag` keep
the flag.

## Consequences

- Composites compare, hash and test `is` as CPython's do, and their
  `repr`, `str`, `name`, iteration and errors match CPython 3.12 to 3.15
  for every value.
- A program that combines many distinct values keeps a composite for
  each, as CPython does.
- A member of several bits or of `0` is planned for 0.13 (CLS-47).

## Related requirements

CLS-44, CLS-47, ADR-0116, ADR-0117, ADR-0093.
