# ADR-0089: Write `map` and `filter` as generic iterator classes of the module

## Status

Accepted

## Date

2026-11-21

## Context

ADR-0078 makes `map` and `filter` iterator objects of the runtime, whose
next entry calls the function through its value (LIB-23). Building it on
2026-11-21 showed that the runtime cannot make that call: the code of a
function value takes and returns values of the types the compiler gave
it, an `int` word, a `float`, a pointer or a tuple held in place, so
only code the compiler writes for those types can call it (ADR-0060).
The runtime would need one entry for each signature, or a step written
by the compiler for each use.

The compiler already writes code for that case. A generic class of the
program with `__iter__` and `__next__` is copied for its type arguments
(ADR-0023), its copies are iterators that `for`, `list`, `next` and
every `Iterator[T]` parameter take (ADR-0065), and an exception
`__next__` raises leaves the object as it was, so the next step goes on.
A class written as below gave, on 2026-11-21, what CPython 3.12.16 to
3.15.2 give for `map` and `filter`, an exception part way included:

```python
class Map[T, R]:
    def __init__(
        self, function: Callable[[T], R], items: Iterator[T]
    ) -> None:
        self.function = function
        self.items = items

    def __iter__(self) -> "Map[T, R]":
        return self

    def __next__(self) -> R:
        return self.function(next(self.items))
```

A call of a generic class solves its type parameters from the types of
its arguments, each typed without an expected type, so the result type
`R` of `map(int, words)` or of a `lambda` is not solved that way: a
built-in or a `lambda` takes its type from the `Callable` it fills.

## Options considered

1. **ADR-0078 as written,** with an entry of the runtime for each
   signature a function value may have. Signatures are open-ended.
2. **A step the compiler writes for each use,** lowered by hand into
   TyIR: a class with the function and the iterators as fields and a
   `__next__` built node by node.
3. **Generic iterator classes the compiler adds to the module,** written
   as Python source as the methods of a dataclass are (CLS-05), and a
   call of `map` or `filter` checked by the checker, which types the
   iterables first and then the function against their item types.

## Decision

Option 3.

- **The classes.** The dataclass expansion adds to a module that calls
  `map` or `filter`, and binds neither name itself, a generic class for
  each form it calls: `map` with one, two or more iterables, `filter`
  with a function, and `filter(None, ...)`. Each holds the function,
  where it has one, and an iterator for each iterable, and its
  `__next__` takes an item from each iterator, calls the function, and
  returns the result, or for `filter` loops until an item, or the
  function's result for it, is true. Their names hold a space, so no
  program names them, and imports with names of the same kind bind
  `Callable` and `Iterator` for their annotations.
- **Names.** Lowering writes each class and its copies with the name
  `map` or `filter` and no module, so `repr` gives `<map object at
  0x...>` and `type()` gives `<class 'map'>`, as in CPython.
- **A call.** The checker types each iterable, makes its iterator as
  `iter()` does, types the function against the item types as the `key`
  of `sorted` is typed (FUN-05, ADR-0077), and makes an instance of the
  copy for the item types and the function's result type. The function
  is evaluated first, then each iterable, as in CPython. A call with too
  few or too many arguments is refused with CPython's text.
- **`strict=`** is accepted for a program whose Python floor is 3.14 or
  later, raising CPython's `ValueError` when the iterables differ in
  length, and refused below it with CPython 3.12's text (ADR-0008).
- **Values.** A `map` or `filter` object is a value a program stores and
  passes. `map` and `filter` themselves as values, such as `f = map` or
  `isinstance(x, map)`, are planned for 1.0 (LIB-25).

## Consequences

- `list(map(int, line.split()))` and `for name in filter(None, names)`
  behave as in CPython, also when the function raises part way.
- The runtime gains nothing; each step is a call of a method and one
  through the function value, as ADR-0078 expected.
- A module's classes are its own: a `map` object of one module and one
  of another have types that are not the same object.
- ADR-0078 is superseded.

## Related requirements

LIB-23, LIB-25, FUN-05, FUN-08, FUN-14, CLS-05, ADR-0008, ADR-0023,
ADR-0060, ADR-0065, ADR-0077, ADR-0078.
