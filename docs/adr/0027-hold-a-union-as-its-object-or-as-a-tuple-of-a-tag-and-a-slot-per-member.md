# ADR-0027: Hold a union as its object, or as a tuple of a tag and a slot per member

## Status

Accepted

## Date

2026-11-13

## Context

ADR-0022 decided how TyPy holds a union such as `int | str` (TYP-14): a
union whose members are all objects is the object's address, and any
other union is a tag word followed by the value, in as many bytes as its
largest member takes, the members overlapping.

Unions of objects are built as ADR-0022 decided. Building the overlapped
layout for the other unions needs a new kind of value in TyIR, in code
generation and in the runtime: one whose object fields are counted, copied
and released according to its tag, since the same bytes hold an `int`
word for one tag and an object's address for another.

TyPy already has a value whose fields are counted by their types: the
tuple, held by value with its items in place. Code generation copies and
releases a tuple field by field; the runtime stores a tuple in a list, a
set or a dict through its descriptor, and counting passes over a null
address and an inline `int` word. An optional tuple is already held as a
tuple of a presence flag and the tuple (ADR-0011).

## Options considered

1. **Overlapped members, as ADR-0022 decided.** A tag word, then the
   largest member's bytes. Every copy, release and store switches on the
   tag, in code generation and in each runtime function that handles an
   item.
2. **A tuple of the tag and one slot for each member.** The union is a
   tuple `(tag, slot 0, slot 1, ...)`: the slot the tag names holds the
   value, and every other slot holds the value that counting passes over,
   the null address, the `int` 0, `0.0` or `False`. The tuple is copied,
   released and stored as any tuple is; only the operations that read the
   value switch on the tag.
3. **Box value members.** Every union is an object, an `int` member a
   heap object of its own. ADR-0022 refused this for the allocation each
   stored `int` costs.

## Decision

TyPy uses option 2 for a union with a member held by value, and keeps
ADR-0022's decision for a union of objects. This record supersedes
ADR-0022 and restates the whole decision.

- **Normal form.** A union is a set of members, so `int | str` and
  `str | int` are one type. A member that another member accepts is
  dropped, and a union of one member is that member.
- **A union of objects** is the object's address, held as the class
  every member derives from when they share one, and otherwise as an
  object whose header says what it is.
- **Any other union** is a TyIR tuple marked as a union: an `int` tag,
  the member's position in the normal form, then one slot for each
  member. `None` makes the union optional, held as ADR-0011 holds an
  optional tuple: a presence flag beside the union.
- **Reading the value** switches on the tag: narrowing reads the slot it
  names, and `str`, `repr`, truth, `type()` and an operation checked for
  each member run the branch for the tag.
- **Containers.** The runtime gains an item kind for a union, whose
  descriptor is the union's tuple. It writes, compares, hashes and tests
  the item as the member its tag names: two members that are numbers
  compare by value, and two members of other types are not equal, as in
  CPython.

## Consequences

- A union with a value member takes eight bytes for the tag and eight
  for each member it can hold, more than the overlapped layout: an
  `int | str` takes 24 bytes in place of 16.
- Code generation and counting need nothing new: the union is a tuple.
  Only the operations that read the value, and the runtime's item kind,
  know the tag.
- Storing an `int` in an `int | str` allocates nothing.
- A later change may overlap the slots without changing the language:
  the layout is internal to TyIR and the runtime.

## Related requirements

TYP-07, TYP-14, ADR-0009, ADR-0011, ADR-0012, ADR-0021, ADR-0022.
