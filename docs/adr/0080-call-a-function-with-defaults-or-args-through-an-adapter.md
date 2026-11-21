# ADR-0080: Call a function with defaults or `*args` through an adapter

## Status

Accepted

## Date

2026-11-20

## Context

A `Callable[[A1, ..., An], R]` value is called with exactly `n`
positional arguments, and since 0.8 TyPy accepts only a function whose
parameters are exactly those (FUN-05, NFR-19). Python programs also pass
functions that take more:

- `apply(total, 2, 3)` with `def total(*values: int) -> int`;
- `lambda *values: sum(values)` where a `Callable[[int, int, int], int]`
  is expected;
- `scale` with `def scale(a: int, by: int = 2) -> int` as a
  `Callable[[int], int]`;
- a function with `**options`, which a `Callable` call fills with an
  empty dict.

CPython 3.12.16 and 3.15.2, checked on 2026-11-20, run each of these, and
the value passed is the function itself: `<function total at 0x...>`.
In TyPy the code of a function value takes its parameters in its own
layout (ADR-0060), and a call with unpacking packs `*args` and `**kwargs`
in the code of the call (ADR-0073).

## Options considered

1. **An adapter for each pair of a function and a `Callable` type,** a
   hidden function of the `Callable`'s parameters that calls the function
   with its defaults, a tuple of the extra values for `*args` and an empty
   dict for `**kwargs`.
2. **One variadic convention for every call through a value,** so any
   function fits. Every call through a value would pack its arguments, a
   cost ADR-0073 kept to calls with unpacking.
3. **A table of entries by arity in each descriptor.** It needs every
   arity a program uses, which separate compilation does not know
   (ADR-0016).

## Decision

Option 1.

- **The fit.** A function fits `Callable[[A1, ..., An], R]` when:
  - its positional parameters take `A1` and onward unchanged, and at most
    `n` of them have no default;
  - each position past its positional parameters passes unchanged as the
    item type of its `*args`, and there is no such position without one;
  - it has no keyword-only parameter without a default;
  - its result passes unchanged as `R`.

  `**kwargs` then receives an empty dict, and a parameter that no position
  reaches takes its default.
- **The adapter.** For each function and `Callable` type that fit only
  this way, the compiler makes a hidden function of the `Callable`'s
  parameters, whose body calls the function as the fit says. For a
  module-level function or a method used through its class it is one
  immortal object; for a closure or a bound method it is a function
  object that captures the original. Its identity key is the original's,
  so `f is total`, `==`, `hash` and `repr` are the original's (ADR-0076).
- **Nothing expected.** Such a function read where no `Callable` is
  expected, such as `f = total`, is E0203 with the help to annotate the
  variable, since its own type has no `Callable` spelling.
- **`lambda *values`.** The item type of `values` is the type of the
  expected `Callable`'s positions past the lambda's other parameters,
  which must agree, else E0409 as for a positional tail (ADR-0073). A
  `lambda` with `**options` is E0203, since a `Callable` passes no
  keywords and nothing gives the values a type.
- **Unchanged.** A function whose parameters fit exactly keeps the direct
  form of 0.8 and needs no adapter.

## Consequences

- Functions with defaults, `*args` and `**kwargs` pass where CPython
  passes them, including bound methods and closures, with CPython's
  identity and text.
- A call through an adapter costs one direct call more, and a closure or
  bound method used this way costs one allocation when the value is made.
- A keyword-only parameter with a default no longer blocks a function
  from being a value, which makes FUN-13 cover defaults as well as
  variadic parameters.

## Related requirements

FUN-13, FUN-05, NFR-19, EXP-14, ADR-0016, ADR-0060, ADR-0073, ADR-0076.
