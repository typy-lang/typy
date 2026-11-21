# ADR-0090: Hold a value of type `object` as an object narrowed by its type ID

## Status

Accepted

## Date

2026-11-21

## Context

TYP-10 rejects `object` as a type, so `def describe(value: object)`,
`list[object]` and `dict[str, object]` are E0403 in 0.10, and so is the
common `def __eq__(self, other: object) -> bool`. Mypy accepts each of these
in strict mode, and the research note "values whose type is known late"
found that every such program is statically typable: an operation that is
not `object`'s own needs `isinstance`, `match` or another narrowing first,
as it does in Java, whose `Object` holds a value of any class and whose
casts and patterns are checked (OI-32).

TyPy holds a value of a closed union as a tag and a slot for each member
(ADR-0027), which cannot hold an open set of types. Every object, however,
already carries a type ID in its header, and an instance points to its
class (ADR-0012), so a value held as an object can always say what it is.

## Options considered

1. **Keep rejecting `object`.** Programs restate the value as a union of
   the types they expect, which works only when the set is known and
   small, and does not match typed Python as mypy reads it.
2. **A tag and a slot for every type of the program.** A union of every
   type is as large as its largest member and grows with the program.
3. **The value as an object, boxed when it is held by value,** as Java
   boxes an `int` into an `Integer` and C# a value type into `object`.

## Decision

Option 3.

- **Representation.** A value of type `object` is the address of an
  object. A `str`, `bytes`, collection, instance, function or any other
  object is held as it is. A value TyPy holds by value is boxed when it
  becomes an `object`: an `int` as an object of the runtime's `int` kind,
  a `float` as an object of a new `float` kind, `True`, `False` and `None`
  as immortal objects, and a fixed tuple as a tuple object. A value is
  unboxed when narrowing gives it a type held by value.
- **What it supports without narrowing.** What `object` defines: `==`,
  `!=` and `is`, `hash`, `str`, `repr`, `format` with an empty
  specification, truth, `type()` and `isinstance`. Every other operation,
  attribute and call is E0404 until the value is narrowed, as mypy
  rejects it.
- **Narrowing.** `isinstance`, a class pattern, `type(x) is C`, `is None`
  and `callable` narrow an `object` as they narrow a union (TYP-07),
  reading the type ID, or the class for an instance.
- **Containers.** `list[object]`, `dict[str, object]` and the other
  collections store `object` items as addresses, and every value that
  fits a member type passes as an `object`.
- **Annotations.** A parameter, variable, attribute, item or result may be
  annotated `object`; an unannotated value still needs a type from its
  first assignment (TYP-02).

## Consequences

- Safety: a value is read as a narrower type only after its type ID or
  class says it is one, so no operation reaches memory of the wrong
  layout, and an `object` holds a reference that the reference count
  keeps alive as any object's.
- Speed: only values typed `object` pay for a box and an indirection;
  every other value keeps its representation. Narrowing costs one load and
  compare.
- The runtime gains a `float` object kind and immortal `True`, `False` and
  `None` objects; equality, hashing and `repr` dispatch on the type ID.
- `typing.cast` can be checked against the same type ID (ADR-0091), and
  `typing.Any` can build on this representation (ADR-0094).

## Related requirements

TYP-23, TYP-10, TYP-02, TYP-07, TYP-14, NFR-07, NFR-19, OI-32, ADR-0012,
ADR-0027.
