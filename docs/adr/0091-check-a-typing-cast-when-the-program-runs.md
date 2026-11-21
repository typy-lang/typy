# ADR-0091: Check a `typing.cast` when the program runs

## Status

Accepted

## Date

2026-11-21

## Context

`typing.cast(T, value)` returns `value` unchanged in CPython, which checks
nothing: a program whose cast is wrong goes on with a value of another
type. TyPy reads a value by the layout its type gives, so trusting a wrong
cast would read memory of the wrong layout, which memory safety forbids
(NFR-07). TYP-10 rejects `cast` in 0.10 (OI-36).

mypyc compiles `cast(str, x)` to a strict check that raises `TypeError`,
and Java's casts throw `ClassCastException`.

## Options considered

1. **Keep rejecting `cast`.** Programs narrow with `isinstance` or
   `match` instead.
2. **Trust the cast, as CPython does.** Not memory safe.
3. **Check the cast and raise `TypeError` when it is wrong,** as mypyc
   does.

## Decision

Option 3.

- `cast(T, value)` compiles to the narrowing of `value` to `T` that
  `isinstance` performs, then gives the value as a `T`.
- When the value is not a `T`, the program raises `TypeError` with the
  text `cast to T found V`, where `T` is the cast's type and `V` the
  value's type, as TyPy writes them in messages.
- A cast to a type the checker already proves, such as `cast(int, 1)`,
  compiles to the value.
- `T` is any type an `isinstance` test can decide: a class, a built-in
  type, a union of them, `object`, and `None`. A cast to a type it cannot
  decide, such as `list[int]` or a `Callable`, is E0404, since checking it
  would mean checking every item.

## Consequences

- A program whose cast is right runs as in CPython. A program whose cast
  is wrong raises where CPython goes on, the one place TyPy knowingly
  differs from CPython, and only for a program its type system calls
  wrong.
- The check costs a type ID compare where the checker cannot prove the
  cast.

## Related requirements

TYP-24, TYP-10, TYP-23, NFR-07, STD-02, OI-36, ADR-0090.
