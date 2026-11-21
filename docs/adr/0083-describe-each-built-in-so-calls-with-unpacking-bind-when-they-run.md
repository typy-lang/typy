# ADR-0083: Describe each built-in so calls with unpacking bind when they run

## Status

Accepted

## Date

2026-11-20

## Context

ADR-0073 binds `*` and `**` in calls of the program's functions, methods
and classes when the program runs, and leaves for 0.10 the calls of
other built-ins, of function values, of methods through a protocol value
and of exception classes, and `**` in a call of `print` (EXP-19). Most
of those callees have no description of their parameters that the
runtime check `typy_rt_call_check` can read: a built-in is checked by its
own rules, and a function value's type names no parameters.

CPython 3.12.16 and 3.15.2, checked on 2026-11-20, show three things.

- **Results that depend on a length.** `max(*xs)` compares the values
  when `xs` holds two or more, but iterates the one value when it holds
  one: `max(*["abc"])` is `'c'` and `max(*[5])` raises `TypeError: 'int'
  object is not iterable`. `zip(*rows)` gives tuples as long as `rows`.
- **Messages.** Built-ins word their errors their own way, such as
  `len() takes exactly one argument (2 given)`, `divmod expected 2
  arguments, got 1` and `dict.get() takes no keyword arguments`. For an
  unexpected keyword 3.15 writes `print() got an unexpected keyword
  argument 'sepx'. Did you mean 'sep'?` where 3.12 writes `'sepx' is an
  invalid keyword argument for print()`; `sorted` writes `sort()` in
  both, without a suggestion in 3.15.
- **Function values** bind by the names of the function they hold:
  `g(**{"a": 1, "b": 2})` works for `g = lambda a, b: ...`, and a missing
  argument names `<lambda>()`.

Exception classes take any arguments as their `args`, and the built-in
ones refuse keywords: `ValueError(**{"a": 1})` raises `TypeError:
ValueError() takes no keyword arguments`, while an empty `**{}` is fine.

## Options considered

1. **A description for each built-in,** read by the existing runtime
   check, with the built-in's own style of message.
2. **A check written by hand for each built-in,** in the code of its
   call.
3. **Refuse unpacking in calls of built-ins,** keeping only `print`.

## Decision

Option 1.

- **Built-ins.** Each built-in function and method of a built-in type
  that accepts unpacking has a description: its positional and keyword
  parameters, which take keywords, and its message style. The runtime
  check reads it and raises CPython 3.15's `TypeError` (ADR-0029), so the
  conformance tests check only the messages every oracle from the floor
  writes alike.
- **Length-dependent results.** `max(*xs)` and `min(*xs)` are accepted
  when the item type of `xs` is not iterable, or is `str`, whose one item
  iterates to items of the same type; otherwise they are E0202, since
  the result's type depends on the length. `zip(*rows)` gives tuples of
  type `tuple[T, ...]`.
- **`print(**options)`.** The values of a `dict[str, V]` may name any
  parameter of `print`. A key naming one whose type `V` does not fit
  raises CPython's own error when the program runs, such as `sep must be
  None or a string, not int`, or `'str' object has no attribute 'write'`
  for a `file` that is a string.
- **Function values.** A call of a function value accepts `*`, and checks
  when it runs that the number of values is the `Callable`'s, with
  CPython's message naming the function the value holds, read from its
  descriptor. `**` through a function value binds by the names of the
  function it holds, which a `Callable` type does not carry, and is
  planned for 1.0 (EXP-21).
- **Protocol methods.** A call of a method through a protocol value
  binds as a call of the protocol's method, whose parameters the
  dispatcher takes (ADR-0041), and checks with that method's description,
  as ADR-0073 checks any method.
- **Exception classes.** A call of an exception class with `*values` gives
  `args` the member `tuple[T, ...]`, beside the fixed tuples of ADR-0034;
  the `OSError` family keeps its fixed tuples (ADR-0035). A built-in
  exception class checks when the program runs that a `**` dict is empty.
  A program exception class with `__init__` binds as any program class
  does (ADR-0073).

## Consequences

- `max(*values)`, `zip(*rows)`, `print(*items, **options)` and
  `ValueError(*details)` run as in CPython.
- The description of each built-in becomes part of the design, and a new
  built-in that takes unpacking adds its row.
- `max(*lists)` with an item type that may be iterated is refused where
  CPython accepts it, because its result has no single type.
- Reading `args` of an exception class that a program creates with `*`
  costs the loop over a `tuple[T, ...]`, as ADR-0034 reads its tuples.

## Related requirements

EXP-19, EXP-17, EXP-21, EXC-11, EXC-15, ADR-0029, ADR-0030, ADR-0034, ADR-0035,
ADR-0041, ADR-0060, ADR-0073.
