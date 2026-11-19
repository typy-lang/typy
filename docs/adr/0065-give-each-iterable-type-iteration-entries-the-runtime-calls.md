# ADR-0065: Give each iterable type iteration entries the runtime calls

## Status

Accepted

## Date

2026-11-19

## Context

Up to 0.8 a value of type `Iterator[T]` is one of the runtime's iterator
objects (ADR-0014), and a value of type `Iterable[T]` is a built-in
collection or iterator under the type ID 16, which the runtime turns into
an iterator by reading the object's header. A class with `__iter__` and
`__next__` is iterated by its own class type, but an instance of it
given where an `Iterator[int]` is expected is E0404, though `typing`
reads `Iterator` and `Iterable` as protocols that such a class satisfies.

0.9 adds two kinds of iterator a value of these types must hold: a
generator (ADR-0063) and an instance of a class with `__iter__` and
`__next__` (TYP-20). The type of the value names neither the kind nor the
class, and a library's function that takes an `Iterator[T]` cannot know
the classes of the programs that call it.

## Options considered

1. **Iteration entries in each type.** The type object of each kind of
   built-in iterator and iterable, of the generator type, and of each
   class of the program with `__iter__` or `__next__` holds a pointer to a
   function that advances an iterator of it, or makes one; `next()`,
   `iter()` and `for` over an `Iterator[T]` or an `Iterable[T]` call
   through the object's type.
2. **A union of every iterator class of the program,** checked member by
   member (TYP-14). Each function that takes an `Iterator[T]` would test
   every class of the program, a library could not, and each new class
   would change code that does not mention it.
3. **A copy of each function for each kind it receives,** as generic
   functions have (ADR-0023). A list of iterators of several kinds could
   then not exist.

## Decision

Option 1.

- **The entries.** A type object carries a next entry, which advances an
  iterator and gives its item or says it is exhausted, and an iter entry,
  which gives an iterator of an iterable. The runtime fills them for its
  own iterators and collections, as `typy_rt_iterable_iterator` and the
  iterator objects of ADR-0014 do today; the generator type's next entry
  resumes the generator (ADR-0063).
- **A class of the program** with `__next__` gets a next entry the
  compiler emits, which calls `__next__` and reads a `StopIteration` it
  raises as exhaustion, as `for` over such a class does today; one with
  `__iter__` gets an iter entry that calls it.
- **Which classes fit.** A class whose `__next__` returns `T`, and whose
  `__iter__` returns itself, is an `Iterator[T]`; a class whose `__iter__`
  returns an `Iterator[T]` is an `Iterable[T]` (TYP-20). An item must
  pass as it is, as a function fits a `Callable` only when nothing is
  converted (NFR-19): a class whose `__next__` returns an `int` is no
  `Iterator[int | None]`.
- **A call through an entry** passes the item with a reference of its
  own, and a `StopIteration` its `__next__` raises is released, as
  ADR-0014 does for a class in a `for` loop.
- **Static kinds stay inline.** A `for` over a value whose type names its
  kind, such as a `list` or a class, keeps its direct form; only a value
  of type `Iterator[T]` or `Iterable[T]` goes through the entries.

## Consequences

- A function that takes an `Iterator[T]` or an `Iterable[T]` accepts a
  built-in iterator, a generator and an instance of any fitting class,
  also of a program that uses it as a library.
- A step through an entry is one indirect call; a loop over a `list` is
  as fast as before.
- A class's type object grows by two pointers when it defines `__iter__`
  or `__next__`.
- An `Iterable[T]` tested against `Iterator` in `isinstance` answers from
  the object's type, so LIB-20's test of it no longer waits for 1.0.

## Related requirements

TYP-15, TYP-20, LIB-20, NFR-19, FUN-07, ADR-0014, ADR-0023, ADR-0063.
