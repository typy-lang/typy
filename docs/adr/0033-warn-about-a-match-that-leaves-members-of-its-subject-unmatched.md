# ADR-0033: Warn about a match that leaves members of its subject unmatched

## Status

Accepted

## Date

2026-11-15

## Context

ADR-0017 decided that TyPy warns where CPython 3.12 to 3.15 give a
`SyntaxWarning` while compiling, and nowhere else, so that style checks
stay with linters. CST-07 asks for one warning CPython never gives: a
`match` whose cases miss part of the subject's type.

TyPy knows each variable's type when it compiles, and a union, an
optional or a `bool` has a fixed set of members. A `match` over such a
subject is often written to handle each member, as an `if` chain over
`isinstance` tests would be, and a case forgotten there is a mistake the
type of the subject shows. Pyright reports it under
`reportMatchNotExhaustive`, and mypy under its optional `exhaustive-match`
error code, so Python developers who use a type checker already see it.

The same analysis decides whether a `match` can end without any case
running. A function that returns a value from every case of a complete
`match` must not be refused with E0405 for reaching its end.

## Options considered

1. **Warn where CPython warns, and about a `match` that leaves members of
   its subject's type unmatched.** One warning beyond CPython's, about the
   types TyPy checks, and none about style.
2. **Warn only when the project asks**, through `typy.toml` or a flag.
   The default would keep ADR-0017 exactly, at the cost of an option and
   its tests, and the mistake would go unreported for most programs.
3. **No warning.** Keep ADR-0017, and use the analysis only for E0405.
   CST-07 would be rejected.

## Decision

TyPy will use option 1, and this record supersedes ADR-0017, whose rules
otherwise stand.

- **W0401** is a `match` whose subject's type has two or more members,
  where some member is matched by no case without a guard. The members are
  each member of a union, `None` of an optional, and `True` and `False`
  of a `bool`. The warning names the members left, and its help offers
  `case _:` for the rest.
- A case covers a member when its pattern matches every value of it: a
  capture or `_`; `None`, `True` or `False` for the member of that value;
  a class pattern whose sub-patterns are captures or `_`, for a member
  every value of which is an instance of the class; `[*rest]` for a list
  or tuple, and captures for a tuple of their number; `{}` or `{**rest}`
  for a dict; and an `as` or `|` pattern through its parts. A case with a
  guard covers nothing, since the guard may be false.
- A `match` whose cases cover every member cannot end without running a
  case, so a function that returns from each case does not reach its end.
- A `match` whose checking reported an error gets no warning, so one
  cause gives one diagnostic.
- A subject of one member, such as `int` or `str`, never gets the warning:
  its values cannot be listed, and `case _:` is the only way to cover
  them.

## Consequences

- A forgotten case of a union is reported where it is written, as a type
  checker reports it, and `--deny-warnings` makes it an error.
- A program that is clean under `python -W error` may get W0401; this is
  the one warning that is not CPython's.
- The analysis reads patterns without their values, so a set of cases
  that covers a member only together, such as `case []:` and
  `case [x, *rest]:` for a list, is not seen as complete. Such a `match`
  needs `case _:` to end a function that returns a value.
- Further warnings of TyPy's own need a record of their own, as this one
  did.

## Related requirements

CST-06, CST-07, DIA-07, DIA-08, TYP-01, ADR-0017.
