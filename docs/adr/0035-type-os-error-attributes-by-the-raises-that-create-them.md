# ADR-0035: Type OSError attributes by the raises that create them

## Status

Accepted

## Date

2026-11-15

## Context

EXC-14 gave `OSError` and its subclasses CPython's reading of two to five
arguments as `errno`, `strerror`, `filename`, `winerror` and `filename2`,
with the attributes typed `int | None` and `str | None`. To keep those
types, it refused other argument types, more than five arguments, and two
or more through `super().__init__`, and EXC-16 plans them for 0.5.

CPython takes any object for each of them. `OSError("a", "b")` has
`errno` `'a'` and is written `[Errno a] b`; `OSError(1, 2, 3, 4, 5, 6)`
reads none of its arguments and has every attribute `None`; and a subclass
with an `__init__` of its own has its arguments read by `OSError.__init__`
when `super().__init__(...)` calls it, not by `OSError.__new__`. So the
type of `errno` depends on how the program creates its exceptions, as the
type of `args` does, which ADR-0034 infers from every creation.

One corner depends on a value known only when the program runs. CPython's
`OSError.__init__` reads a number as the third argument of an exception
whose class is exactly `BlockingIOError` as `characters_written`, not as
the file name. `OSError` itself becomes `BlockingIOError` when `errno` is
`EAGAIN`, so `OSError(code, "x", 3)` has `filename` `3` or none depending
on `code`, and `OSError(11, "x", 3.5)` raises `TypeError` while
`OSError(2, "x", 3.5)` does not.

## Options considered

1. **Infer the attributes from the raw arguments of every creation**, as
   ADR-0034 does for `args`. Each creation of an `OSError` class records
   the tuple of all its arguments, not only those `args` shows, and each
   attribute is the item its position gives in each tuple, or `None`.
2. **Keep the attributes' EXC-14 types**, and refuse an argument that
   does not fit them. Every attribute keeps one type with no inference,
   but EXC-16 stays unmet.
3. **Give the attributes the type of `args`' items**, the union of every
   item type. Simpler to infer, but `errno` of `OSError(2, "x")` would be
   `int | str`, and a program could not add 1 to it.

## Decision

TyPy uses option 1.

- **Raw arguments.** `exception_arguments::Creations` keeps, for each
  creation, the tuple of every argument the exception holds, and gives
  `args` from it by CPython's rule: a file name that is not `None` keeps
  only `errno` and `strerror`. `OSError`'s own text for a program's
  exception is written when `str` asks for it, from the items of any
  type, as CPython's `OSError_str` does, and the runtime gives the class
  for `errno` only when the first argument is an `int` or a `bool`.
- **The attributes.** For each layout of the raw arguments, `errno` and
  `strerror` are the first two items of two to five, `filename` the third
  of three to five, and `filename2` the fifth of five when the third is
  not `None`; any other layout gives `None`. Lowering tests the layout of
  the exception's raw arguments against each, as it does for `args`, and
  reading one of them makes the program checked again, as reading `args`
  does.
- **A class with its own `__init__`.** Its exception starts with no
  arguments, and `super().__init__(...)` sets them, read as above.
- **The runtime's own `OSError`.** An error of a file opened from a
  descriptor keeps the descriptor as its third argument, so `filename` is
  the `int` CPython gives.
- **A number as the third argument** of `OSError` or `BlockingIOError`,
  whose class may be exactly `BlockingIOError` when the program runs, is
  E0201, planned for 1.0 with `characters_written` (EXC-18). A class
  derived from either, or another subclass, reads it as the file name.

## Consequences

- `OSError` takes arguments of any type, more than five of them, and two
  or more through `super().__init__`, with CPython's attributes and text.
- `errno` of a program that creates `OSError` only with an `int` and the
  runtime's raises stays `int | None`; one that also creates
  `OSError("a", "b")` has `errno` of `int | str | None`.
- A program that reads `errno`, `strerror`, `filename` or `filename2` is
  checked two or more times, as one that reads `args` is.
- `typy_rt_os_error_errno` and `typy_rt_os_error_text` are no longer
  needed, and are removed.

## Related requirements

EXC-14, EXC-15, EXC-16, EXC-18, CLS-14, ADR-0019, ADR-0034.
