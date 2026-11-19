# ADR-0067: Type what a `yield` receives as the send type or `None`

## Status

Accepted

## Date

2026-11-19

## Context

In `received = yield value`, the `yield` gives what resumed the
generator: the value of `send(value)`, or `None` when `next()`, a `for`
loop or `send(None)` resumed it (SMT-11). `Generator[Y, S, R]` names
`S`, the type `send` takes (FUN-07). CPython 3.12.16 and 3.15.2, checked
on 2026-11-19, give `None` to the `yield` of every step that `next()`
starts, whatever `S` is, and accept `send(None)` to start a generator.

Python's type checkers give the `yield` the type `S`. A generator of
`Generator[int, int, None]` that a `for` loop drives then holds `None`
in a variable typed `int`, which the checker never reports. TyPy compiles
a variable of type `int` as a machine word that cannot hold `None`, so
it cannot follow them without a check at run time or an unsound value.

## Options considered

1. **`S`, as type checkers have it,** with a check at run time that
   raises when `None` arrives. A program that type checkers accept then
   fails in TyPy where CPython goes on.
2. **`S | None`,** or `S` itself when `S` already admits `None`. The
   program tests for `None` before it uses the value, as it must in
   CPython for every step a `for` loop or `next()` starts.
3. **Require `S` to admit `None`,** refusing `Generator[int, int, None]`
   for a generator that assigns a `yield`. Every program would write
   `Generator[int, int | None, None]`, and `send` would accept `None`
   where the generator expects a number.

## Decision

Option 2. The `yield` of `target = yield value` gives `S | None`, which
is `S` when `S` is `None` or an optional or union type with `None` in it.
`send` takes `S` only, as type checkers have it; `send(None)` to a
generator whose `S` does not admit `None` is E0404, whose help names
`next(generator)` to start it. An assignment of the value to a variable
of type `S` is E0402, whose help says that `next()` and a `for` loop
send `None` and asks for `S | None` and a test.

## Consequences

- What a program receives is sound: no variable holds `None` that its
  type does not admit, and the result matches CPython at run time.
- A program written for a type checker may need `received: S | None`
  and a test for `None` where the checker accepted `S`.
- `send` can pass its value to the generator's frame as the bits of an
  `S`, with a flag that says whether it is `None`, which the runtime
  needs for CPython's `TypeError` of a value sent to a generator not
  started.

## Related requirements

SMT-11, FUN-07, TYP-06, ADR-0011, ADR-0063.
