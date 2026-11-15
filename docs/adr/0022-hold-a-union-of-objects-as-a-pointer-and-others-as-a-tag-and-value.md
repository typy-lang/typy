# ADR-0022: Hold a union of objects as a pointer and others as a tag and value

## Status

Superseded by ADR-0027

## Date

2026-11-13

## Context

TYP-14 asks for annotations that join two or more types other than
`None` with `|`, such as `int | str`, and TYP-07 for narrowing them with
`isinstance`. Until 0.4 the only union is `T | None`, held in a value
`T` never takes (ADR-0011): the word 0 or the null address, or a
signalling NaN for a `float`.

A value of each type has one of two shapes:

- **By value.** An `int` is a 64-bit word, a `float` a double, a `bool`
  one bit, and a tuple its items laid out in place.
- **An object.** A `str`, `bytes`, list, set, dict, iterator or instance
  is a pointer to an object whose header holds its type ID; an instance
  also points to its class (ADR-0012).

A union needs to know at run time which member it holds: `isinstance`
reads it, and writing, comparing and hashing a union inside a container
depend on it.

## Options considered

1. **Box every member.** A union is always a pointer to an object, as in
   CPython; an `int` member is boxed into a heap object. An
   `int | str` costs an allocation for each `int` stored in it.
2. **A tag and a value, always.** A union is a tag word naming the
   member, then the value. A union of objects carries a tag the object's
   header already holds.
3. **The object for a union of objects, a tag and a value otherwise.**
   A union whose members are all objects is the pointer, and its member
   is read from the header and the class. Any other union is a tag word
   and the value.

## Decision

TyPy uses option 3, which is option 2 without the tag an object already
carries.

- **Normal form.** A union is a set of members, so `int | str` and
  `str | int` are one type. A member that another member accepts is
  dropped: `bool | int` is `int`, and `Shape | Circle` is `Shape` when
  `Circle` derives from `Shape`. A union of one member is that member.
- **A union of objects** is the object's address; `None` in it is the
  null address, as in ADR-0011. Widening a member to the union is free.
- **Any other union** is a tag word, then the value in as many bytes as
  its largest member takes: 16 bytes for a union of scalars and objects.
  The tag is the member's position in the normal form, and `None` in it
  is a tag of its own.
- **Narrowing.** `isinstance`, `is None` and `is not None` narrow a
  union in the branch where the test holds and in the code after a
  branch that cannot fall through, as type checkers do (TYP-07).
- **Operations.** An operation, attribute or method applies to a union
  only when it applies to every member, as type checkers require; a
  method every member has runs by a switch on the member.
- **Containers.** The runtime gains an item kind for a tagged union,
  which writes, compares and hashes the item as its member does, so a
  `list[int | str]` prints and sorts as in CPython.

## Consequences

- An `int | str` takes 16 bytes in a variable and in a container, and
  storing an `int` in it allocates nothing.
- A union of classes, the common case with inheritance, is one pointer,
  and its methods dispatch through ADR-0021's table when the members
  share a base, or by a switch when they do not.
- Each union with a value member adds a tuple descriptor of its tag and
  largest member, shared as tuple descriptors are (ADR-0009).
- Ordering, equality and hashing across members follow CPython: an
  `int` and a `float` compare by value, an `int` and a `str` raise
  `TypeError` when ordered.

## Related requirements

TYP-07, TYP-14, ADR-0009, ADR-0011, ADR-0012, ADR-0021.
