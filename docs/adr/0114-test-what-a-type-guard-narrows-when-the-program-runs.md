# ADR-0114: Test what a type guard narrows when the program runs

## Status

Accepted

## Date

2026-11-23

## Context

TYP-31 accepts a function whose result is `typing.TypeGuard[T]` or
`typing.TypeIs[T]`: a test of a call of it narrows its first argument to
`T` where the call is true, and for `TypeIs` to the rest of the argument's
type where it is false. CPython runs the function and trusts its result;
a type checker trusts the annotation.

TyPy holds a value by the layout its type gives, so a variable narrowed
from `object` to `int` is read as an `int`. A guard whose body returns
`True` for a `str` would make TyPy read a `str` as an `int`, which is no
mistake in Python's sense but a wrong read of memory in TyPy's.

## Options considered

1. **Trust the guard,** as type checkers do. A wrong guard reads memory
   wrongly.
2. **Narrow only what a test of the value's class bears out:** run the
   call, and test the argument as `isinstance` would against `T`. A
   `TypeGuard` call that returns `True` for a value the test does not find
   to be a `T`, and a `TypeIs` call whose result differs from the test,
   raise `TypeError` that names the function.
3. **Narrow by the test alone and ignore the call's result.** The program
   then takes branches CPython does not.

## Decision

Option 2. A call of a guard whose first argument is a variable becomes the
`isinstance` test of that variable against `T`, with the call as its
`GuardCall`; the call's result is the test's result, and the facts of the
`isinstance` test narrow the variable. `T` is a type a test of the class
tells apart: a number, `str`, `bytes`, `None`, a range, a slice, or an
instance of a class that is neither generic nor a protocol, or a union of
these.

## Consequences

- A guard that tells the truth behaves as in CPython, at the cost of one
  class test for each call.
- A guard that does not raises `TypeError` where CPython would go on, so
  a narrowed read never takes a value of another type.
- A guard of a type no class test tells apart, such as `list[int]`, and a
  method whose result is a guard are planned for 0.13 (TYP-37).

## Related requirements

TYP-31, TYP-37, TYP-07, TYP-24, ADR-0091.
