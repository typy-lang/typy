# ADR-0111: Leave code that cannot run unchecked and uncompiled

## Status

Accepted

## Date

2026-11-23

## Context

TyPy checked every statement of a body, also one that cannot run: a
statement after `return`, `raise`, `break` or `continue`, and a branch an
`isinstance` test rules out by the types. A mistake there was an error,
so a program CPython runs was refused:

<!-- typy-expect: accepted -->
```python
def label(value: int) -> str:
    if isinstance(value, str):
        return value.upper()
    return str(value)


print(label(3))
```

mypy and pyright leave such code unchecked by default. TYP-29 checks an
overloaded function's body against each of its signatures, and against
the signature for `int` the branch for `str` above cannot run; checking it
there would refuse every overload whose body tests its argument's type.

## Options considered

1. **Keep checking all code.** Programs CPython runs stay refused, and
   TYP-29 cannot check a body against each signature.
2. **Check all code, and report errors only where code can run.** The
   checker would carry the types of code that cannot run, which `isinstance`
   leaves without a type to narrow to.
3. **Leave code that cannot run unchecked and uncompiled.** A block stops
   at the first statement no path reaches, and the branch of an `if`, an
   `assert` or a `case` whose test the types decide is not checked.

## Decision

Option 3. What decides that code cannot run is what TyPy already knows
for certain: a statement after one that leaves the block, the constants
`True` and `False` as a test, and an `isinstance` test, or a class
pattern, whose outcome the value's static type fixes. A value's static
type holds for every value when the program runs, so such a branch never
runs. The branches of a conditional expression and the operands of `and`
and `or` are expressions, and stay checked.

## Consequences

- A program with code that cannot run, which CPython runs, is accepted
  with CPython's output, and the code is not compiled.
- A mistake in code that cannot run is not reported, as mypy does by
  default.
- An overloaded function's body is checked against each signature with
  the branches for the others' types left out (TYP-29).

## Related requirements

TYP-34, TYP-07, TYP-29, CST-06.
