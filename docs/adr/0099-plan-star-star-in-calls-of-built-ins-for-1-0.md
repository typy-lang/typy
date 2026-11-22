# ADR-0099: Plan `**` in calls of built-ins for 1.0

## Status

Accepted

## Date

2026-11-22

## Context

ADR-0083 lets a call of a built-in function or method unpack `*` of any
iterable and `**` of any `dict[str, V]`, binding when the program runs.
Release 0.11 delivers the `*` half for every built-in function, every
method of a built-in type and every function of `math` and `sys`, and
`**` for `print`, a method through a protocol value and an exception
class (EXP-22).

`**` for the other built-ins needs more than a description of their
parameters. A key names one of about 60 keyword parameters, and each
value must be checked against its parameter when the call runs, with the
`TypeError` CPython 3.15 raises for that parameter, which each built-in
words its own way, such as `'str' object cannot be interpreted as an
integer`. Checking the dict's value type against every keyword a key may
name, as EXP-17 does for the program's functions, would refuse nearly
every real call: no one value type fits both `key` and `reverse` of
`sorted`. Such calls are rare in Python code, and NFR-21 still asks for
work on five benchmark programs in 0.11.

## Options considered

1. **Bind each keyword when the call runs,** with a check of each value
   against its parameter and CPython's message, as ADR-0083 planned, in
   0.11.
2. **Check the dict's value type against every keyword a key may name,**
   as EXP-17 does, in 0.11.
3. **Plan `**` in calls of built-ins other than `print` for 1.0,** as a
   requirement of its own.

## Decision

Option 3, chosen by the project on 2026-11-22. EXP-22 keeps `*` in every
call ADR-0083 names and `**` in a call of `print`, of a method through a
protocol value and of an exception class. EXP-24, planned for 1.0, takes
`**` in a call of any other built-in function or method and of a function
of a built-in module, bound as ADR-0083 decides.

## Consequences

- `sorted(values, **options)`, `open(path, **options)` and the like are
  E0201, planned for 1.0 (EXP-24), until then.
- `**` of a dict display whose keys are `str` literals still binds as the
  keywords it writes out (EXP-19).
- 0.11's remaining work goes to the benchmark programs of NFR-21.

## Related requirements

EXP-22, EXP-24, EXP-19, EXP-17, NFR-21, ADR-0083.
