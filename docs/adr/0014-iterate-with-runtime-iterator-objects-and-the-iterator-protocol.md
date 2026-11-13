# ADR-0014: Iterate with runtime iterator objects and the iterator protocol

## Status

Accepted

## Date

2026-11-11

## Context

Release 0.2 compiles `for` over the built-in iterables (CST-03) without
an iterator object: the loop keeps the sequence and its position in
hidden slots and reads the next item inline. No program can hold an
iterator as a value.

Release 0.3 adds `for` over a class with `__iter__` and `__next__`, ended
when `__next__` raises `StopIteration` (CST-08), and `iter()` and `next()`,
with and without a default, for the built-in iterables (CST-09). Both need
iteration as a protocol with values: `iter(xs)` gives an object that
`next()` advances, and a class's `__iter__` may return one. `Iterator[T]`
and `Iterable[T]` annotations arrive with imports in step 5 of the release
(TYP-15).

CPython's iterators define the behaviour to keep: a list iterator reads
the list's length again at each step, so items appended are visited; a
set or dict iterator raises `RuntimeError` when its source changes size
while it runs; an exhausted iterator raises `StopIteration` with no
arguments.

## Options considered

1. **Compile `iter()` and `next()` inline**, as the loops of 0.2 are, with
   the state in hidden slots. A value that leaves the function, or is
   kept in a list, has nowhere to keep its state.
2. **A runtime iterator object.** `iter()` makes a heap object that holds
   its source and its position; `next()` calls the runtime, which steps
   it with the source's own rules. Loops of 0.2 keep their inline form.
3. **Run every `for` loop through iterator objects.** One mechanism, but
   each loop allocates an object and calls the runtime at each step,
   where 0.2 reads items inline.

For a class, `for` and `next()` call its `__next__` and must stop at
`StopIteration` and nothing else. The error path of the call either goes
to the end of the loop, or to `next()`'s default, when the exception is a
`StopIteration`, or goes on as any other exception does.

## Decision

TyPy uses option 2.

- An iterator object is a heap object under its own type ID: its kind
  (list, tuple, `str`, `bytes`, set, dict keys, dict values or dict
  items), a reference to its source, and its position, with the length
  and count a set or dict iterator checks. Its type object is CPython's:
  `list_iterator`, `tuple_iterator`, `str_ascii_iterator` or
  `str_iterator`, `bytes_iterator`, `set_iterator`, `dict_keyiterator`,
  `dict_valueiterator` and `dict_itemiterator`.
- Advancing an iterator is one runtime call that reports whether an item
  is there, after the source's checks; a second call gives the item, or
  the key or the value of a dict entry, with a reference of its own. An
  item of a dict's items view is a tuple the compiler builds from the key
  and the value, as the loops of 0.2 do.
- The checker types an iterator as `Type::Iterator` of its kind and item
  type. `iter()` of an iterator gives the iterator itself. `next(it)`
  gives the item type; `next(it, default)` gives it when the default has
  the item type, or the optional item type when the default is `None`.
- A class may define `__iter__`, taking only the instance, and
  `__next__`, taking only the instance and returning the item type.
  `for`, `iter()` and `next()` call them. `for` over a class calls
  `__iter__` once, then `__next__` until a `StopIteration`, which it
  releases. `for` over an iterator object steps it until it is exhausted.
- The loops of 0.2 over built-in iterables keep their inline form.

## Consequences

- An iterator can be stored, passed and returned like any value, and
  `next()` on it behaves as CPython's, including the errors of a source
  that changes size.
- Each `iter()` allocates one object, and each `next()` is two runtime
  calls; `for` over a list allocates nothing, as before.
- Catching `StopIteration` on the error path of `__next__` costs nothing
  until it is raised, as every exception (ADR-0004).
- Until imports arrive in step 5 (TYP-15), a variable holds an iterator
  by inference only, and `__iter__` returns the instance itself or an
  instance of an iterator class.

## Related requirements

CST-03, CST-08, CST-09, TYP-15, EXC-02, NFR-01.
