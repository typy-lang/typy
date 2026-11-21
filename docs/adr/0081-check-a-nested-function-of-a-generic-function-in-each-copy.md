# ADR-0081: Check a nested function of a generic function in each copy

## Status

Accepted

## Date

2026-11-20

## Context

A generic function, and a function with a protocol parameter, which
compiles as a generic function, is checked once with opaque type
parameters, once for each constraint, and then compiled as one copy for
each set of type arguments the program reaches (ADR-0023). Up to 0.9 a
nested function, a `lambda` or a generator expression inside such a
function, and a nested function with type parameters of its own, are
E0201 (FUN-10).

CPython 3.12.16 and 3.15.2, checked on 2026-11-20, run each of these:

- `pair_with[T](first: T)` returning a nested `make(second: T)` gives
  `(1, 2)` for `pair_with(1)(2)`; the nested function's `__qualname__` is
  `pair_with.<locals>.make`, and each call makes a new function object.
- A nested `def apply[T](...)` inside an ordinary function, called with
  two types, runs for each; its `__type_params__` is `(T,)`.
- `nonlocal`, `lambda` and generator expressions inside a generic
  function behave as in any function.

Two defects stand in the way. Substituting type arguments skips the
parameters and result of a `Callable` and the types of a `Generator`, so
`keep(1, inc)` with `f: Callable[[T], T]` is refused with E0404, and a
generic generator that yields `T` with E0405. A nested function that
takes or returns `T` needs both.

## Options considered

For a nested function inside a generic function:

1. **Check and compile it in each copy of the outer function,** with the
   copy's types, as the rest of the copy's body is.
2. **Make the nested function generic in the outer type parameters,** and
   copy it separately. Its captures would have types that depend on the
   outer copy, so each of its copies would still belong to one outer copy.

For a nested function with type parameters of its own:

1. **One function object per run of its `def`,** holding its captures and
   defaults, shared by every copy, each copy's code reading the captures
   by the same layout.
2. **One object per copy,** made when the `def` runs. A `def` would
   allocate as many objects as the program has copies of it.

## Decision

Option 1 in both cases.

- **Substitution.** Substituting and inferring type arguments goes into
  every type that can name a type parameter: the parameters and result of
  a `Callable`, the three types of a `Generator`, the item of an
  `Iterator` or `Iterable`, and the members of a union, a tuple and a
  container.
- **Nested code in a generic function.** A nested function, a `lambda` or
  a generator expression inside a generic function, or inside a function
  with a protocol parameter, is checked and compiled as part of each copy
  of the outer function, with the copy's types. The nested functions made
  while the generic body is checked with opaque types, and once per
  constraint, report their errors and are then dropped, so only copies
  make code.
- **Names.** A nested function keeps the qualified name Python gives it,
  `pair_with.<locals>.make`; the copy's name with its type arguments,
  such as `pair_with[int]`, is only the name its code is linked under.
- **A nested function with type parameters.** Its `def` makes one
  function object that holds its captures and defaults, whose layout does
  not depend on its own type parameters. A call by name picks the copy
  for the call's type arguments and passes it the object. As a value it
  is E0202, as a module-level generic function is.
- **Groups.** A generic member of a group of nested functions that call
  one another shares the group's environment, which every copy reads by
  the same layout (ADR-0061).
- **Bound.** Copies of nested functions count toward the bound of 64
  nested copies (E0428).

## Consequences

- Closures, `lambda` and generator expressions work inside generic
  functions and functions with protocol parameters, with CPython's
  names.
- Each copy of a generic function carries its own copy of each nested
  function, so code size grows with the copies, as for any generic code.
- Generic decorators, whose wrapper is a nested function that takes and
  returns the decorated function's types, can be written (FUN-06,
  ADR-0082).

## Related requirements

FUN-10, FUN-04, FUN-06, GEN-01, GEN-03, ADR-0023, ADR-0028, ADR-0060,
ADR-0061, ADR-0082.
