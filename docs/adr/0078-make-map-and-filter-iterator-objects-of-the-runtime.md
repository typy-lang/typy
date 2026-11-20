# ADR-0078: Make `map` and `filter` iterator objects of the runtime

## Status

Accepted

## Date

2026-11-20

## Context

`map(function, iterable, ...)` and `filter(function, iterable)` are the
built-ins that take a function value most often after `sorted` (LIB-23).
CPython 3.12.16 and 3.15.2, checked on 2026-11-20, agree on what they do:

- Both are lazy iterators: the function runs once for each `next`.
  `map` with several iterables stops at the shortest, and `filter` keeps
  the items whose result is true, or the true items themselves when the
  function is `None`.
- Both keep going after the function raises. `map(int, ["1", "x", "3"])`
  gives `1`, then raises `ValueError`, and the next `next` gives `3`. The
  generator expression `(int(s) for s in ["1", "x", "3"])` gives `1`,
  raises `ValueError`, and is then finished.
- `repr` writes `<map object at 0x...>` and `<filter object at 0x...>`.
- `map(f, a, b, strict=True)` checks that the iterables have one length
  from CPython 3.14; CPython 3.12 and 3.13 raise `TypeError: map() takes
  no keyword arguments`.

TyPy already has iterator objects of the runtime with iteration entries
(ADR-0014, ADR-0065), and generators compiled as resumable functions
(ADR-0063). `enumerate`, `zip` and `reversed` are taken by `for` only and
are not values before 1.0 (SRS 6.2).

## Options considered

1. **Compile `map` and `filter` as generator expressions.** No new runtime
   code, but a generator is finished once its body raises, so the program
   would stop where CPython's continues.
2. **Compile them inline in `for` only,** as `zip` is. A `map` could not
   be passed, stored or turned into a list, its common uses.
3. **Iterator objects of the runtime,** each holding its function value
   and the iterators of its iterables, with a next entry that calls the
   function through its value once per step.

## Decision

Option 3.

- **The objects.** A `map` holds the function value and one iterator for
  each iterable; a `filter` holds the function value, or none for `None`,
  and one iterator. Each has a type object of the runtime named `map` or
  `filter` with the next and iter entries of ADR-0065, so a `for`, `list`,
  `next` and any `Iterator[T]` or `Iterable[T]` parameter take it.
- **Types.** `map(f, xs, ys)` is an `Iterator[R]` when `f` is a
  `Callable[[X, Y], R]` and the items of `xs` and `ys` pass to `X` and `Y`
  unchanged; a built-in or a type as `f` takes that `Callable` type
  (ADR-0077). `filter(f, xs)` is an `Iterator[T]` of the items `T` of
  `xs`, with `f` a `Callable[[T], R]` for any `R` whose truth TyPy tests,
  or `None`.
- **A step.** The next entry advances each iterator, calls the function
  through its value, and returns the result or the exception. An
  exception leaves the object as it was after the items it read, so the
  next step goes on, as in CPython.
- **`strict=`** is accepted for a program whose Python floor is 3.14 or
  later, and is E0202 with the floor to set below it (ADR-0008).
- **Messages.** A call with too few arguments is refused when it compiles,
  with CPython 3.15's text, such as `map() must have at least two
  arguments.`

## Consequences

- `list(map(int, line.split()))` and `for name in filter(None, names)`
  behave as in CPython, also when the function raises part way.
- `map` and `filter` objects are values a program stores and passes,
  where `zip` and `enumerate` objects still wait for 1.0.
- Each step is an indirect call through the function value and one
  through the iterator's entry; a loop that needs neither can still be
  written as a `for` or a comprehension.

## Related requirements

LIB-23, FUN-08, FUN-14, CST-09, ADR-0008, ADR-0014, ADR-0063, ADR-0065,
ADR-0077.
