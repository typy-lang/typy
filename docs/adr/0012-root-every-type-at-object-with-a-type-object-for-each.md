# ADR-0012: Root every type at object, with a type object for each

## Status

Accepted

## Date

2026-11-10

## Context

Release 0.2 adds classes with `__init__`, methods, declared attributes,
`__str__` and `__repr__` (CLS-01, CLS-02, CLS-08). Release 0.4 adds single
inheritance with overriding methods (CLS-04), `isinstance` (CLS-09) and
`super()`. In CPython every object refers to its type, every type is an
object whose type is `type`, and every type has a base, up to `object`,
which has none. Inheritance, `isinstance`, method lookup and `super()` all
follow that chain of bases.

The runtime so far has an 8-byte header on every heap object: a reference
count and a 32-bit type ID (NFR-12, ADR-0001). Each operation that handles
an object of any type, such as freeing it or comparing, hashing, testing or
writing a list item, matched the type ID itself, so a type was spread over
five functions and a class had no place to give its own.

## Options considered

1. **A pointer to the type in the header.** This is CPython's layout. The
   header grows to 16 bytes, which NFR-12 does not allow, and every string,
   list and large integer pays for it.
2. **A type ID that indexes a table of type objects.** The header stays 8
   bytes. The runtime's types take fixed IDs, and the program adds one
   entry for each class. The table is a process-wide object that the
   program fills before it runs, and two TyPy libraries in one process
   would give the same IDs to different classes.
3. **A type ID, and for an instance of a class a pointer after the
   header.** The runtime's types keep their fixed IDs and find their type
   objects from them. An instance of a class has one more ID, and the word
   after its header points to its class's type object, which generated code
   emits as a constant. Instances take eight more bytes, as they do in
   CPython; no other object does.

## Decision

TyPy uses option 3. A type object records the type's name, its base, and
the functions that test, hash, compare, order and write its objects;
an absent function means what it means for `object`: every object is true
and equal only to itself, and has no hash, order or text of its own until
the type gives one. `object` is the root and has no base; every type the
runtime defines has `object` as its base, and a class names its base, which
is `object` when the class names none. Type objects are immortal objects
whose type is `type`. The header keeps its 32-bit type ID, and the runtime
finds the type of any object from it, so testing, hashing, comparing and
writing an object each look the type up and call its function.

Freeing is the exception. Every program frees objects, and a lookup there
would link every type object, and through them every function they name,
into a program that only prints a string: a static hello world grew from
under 100 KB to 128 KB, over NFR-01. The runtime therefore frees its own
types by a `match` on the type ID, and only an instance of a class is
freed through its type object.

## Consequences

- Each runtime type is described in one place, and a class plugs its
  `__repr__` or `__str__` into the same lookup that writes a list item.
- The header stays at 8 bytes. An instance of a class has a 16-byte
  prefix, its header and its type, before its attributes.
- A chain of bases is ready for inheritance, `isinstance` and `super()` in
  0.4, which walk it; until then every class has `object` as its base.
- Calling through a function of the type costs an indirect call where a
  `match` on the type ID was a jump. The calls the compiler knows the type
  of keep calling the runtime function directly.
- `type(x)` gives the type object, so it needs no table of names.

## Related requirements

CLS-01, CLS-04, CLS-08, CLS-09, CLS-10, NFR-12, ADR-0001.
