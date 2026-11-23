# ADR-0121: Give copy from the compiler in 0.13

## Status

Accepted

## Date

2026-11-23

## Context

ADR-0119 gives six modules of the standard library in 0.12, each written
in typed Python that CPython also runs: `bisect`, `heapq`, `copy`,
`string`, `textwrap` and `base64`. Five are given that way, `base64` with
the `binascii` functions it uses.

`copy` cannot be written so. `copy.copy` and `copy.deepcopy` take a value
of any type and give one of the same class. CPython finds how to copy a
value from its class, through `__copy__`, `__deepcopy__` and
`__reduce_ex__`, and `deepcopy` keeps a memo of the objects it copied,
keyed by `id`, so an object reached twice is copied once and a cycle
ends. TyPy has no `__reduce_ex__` and no `id`, and typed Python cannot
make an instance of a class without running its `__init__`. typeshed
types the `memo` of `deepcopy`, and the `memo` a class's `__deepcopy__`
takes, as `dict[int, Any]`; `typing.Any` is planned for 0.13 (TYP-25).

## Options considered

1. **Give `copy` from the compiler in 0.12,** as `math` is given: a copy
   method the checker makes for each class, a choice by the value's type,
   and a memo by identity, leaving `memo` and `__deepcopy__` for 0.13.
2. **Give `copy` from the compiler in 0.13,** beside `Any`, so `copy`,
   `deepcopy` with its `memo`, `__copy__` and `__deepcopy__` come
   together.
3. **Keep `copy` as typed Python** and give it for the built-in types
   only.

## Decision

Option 2, chosen by the project on 2026-11-23. LIB-33 holds `copy`,
planned for 0.13. The other decisions of ADR-0119 stand:

- 0.12 gives `bisect`, `heapq`, `string`, `textwrap`, and `base64` with
  the `binascii` functions it uses (LIB-27), and the crate and its
  loading, which `frozendict` also uses (DAT-18).
- 0.13 gives `statistics`, `time` and `random`, with `_typy` (LIB-30),
  and `copy` from the compiler (LIB-33).

## Consequences

- A program that imports `copy` is refused with E0201, naming 0.13 and
  LIB-33.
- Option 3 is not taken: a `copy` that refuses instances of the
  program's classes would fail at its most common use.
- How the compiler copies a value of each type is decided with LIB-33,
  beside TYP-25.

## Related requirements

LIB-27, LIB-33, TYP-25, LIB-30, DAT-18, ADR-0119.
