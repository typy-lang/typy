# ADR-0112: Check a call against the first overload that takes it

## Status

Accepted

## Date

2026-11-23

## Context

TYP-29 accepts `typing.overload` on functions and methods: a call is
checked against the signature it matches, and the body against every
signature. CPython keeps only the implementation, the last `def` of the
name, and runs it for every call; the `@overload` definitions are for type
checkers. About one definition in seven of typeshed's standard library
stubs is an overload, so the typed standard library of LIB-27 and later
needs them.

TyPy compiles one body for one signature. The implementation's signature
is wider than each overload's, such as `int | str` for overloads of `int`
and of `str`, so its result is a union, while a call that matched the
`int` overload should give an `int`.

## Options considered

1. **Compile a copy of the body for each overload**, as generic functions
   are copied. A call runs its overload's copy, whose result has the
   overload's type. The code grows with each overload, and the function
   as a value still needs the implementation.
2. **Compile the implementation once,** check its body against each
   overload, and read a call's result as the overload's. A call of the
   `int` overload passes its argument as the implementation's `int | str`
   and gets back the union, which the check shows holds an `int`.
3. **Check only the implementation's body,** as mypy does, and trust the
   overloads' results.

## Decision

Option 2, with the result tested when the program runs. `check/overloads.rs`
takes the overloads out of the module before decorators are expanded, and
the checker reads their signatures beside the implementation's. A call is
checked against each overload in order, and the first that takes its
arguments gives the types they are checked with and the call's result.
The arguments are passed as the implementation's parameters hold them.

The body is checked once as written, and once more for each overload as a
look: the parameters the overload names have its types and the result is
the overload's. Code the overload's types rule out is not checked
(TYP-34), and a test of literal types decides code in these looks, so an
overload of `Literal['r']` is not checked on the branch for `'rb'`.

The result is read as the overload's through a checked cast, as
`typing.cast` reads a value (ADR-0091). Static types guarantee it, but a
literal type passes for its base type in some places (ADR-0110), and a test
of the value's class costs little. A value of another type raises
`TypeError` instead of being read in the wrong form.

Each overload must fit its implementation: its parameters are the
implementation's of the same names, in the same order and passed alike,
with types the implementation's hold; a parameter it leaves out, or that a
call may leave out, has a default in the implementation; and the
implementation's result holds its result, in a form a test tells apart.
Otherwise E0432.

## Consequences

- One compiled function serves every overload, and a call costs a test
  of its result where the overload's result is narrower than the
  implementation's.
- A body that would return a value of the wrong type for one overload is
  an error, which mypy does not find.
- A parameter an overload leaves out is checked at the implementation's
  declared type, not the value of its default.
- Overloads of static, class and special methods, inside functions and
  blocks, with decorated or generic implementations, in generic and
  protocol classes, on a method a derived class overrides, and calls that
  unpack with `*` or `**` are planned for 0.13 (TYP-35).

## Related requirements

TYP-29, TYP-35, TYP-28, TYP-34, TYP-24, LIB-27.
