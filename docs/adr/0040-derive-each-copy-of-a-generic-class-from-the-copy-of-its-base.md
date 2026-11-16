# ADR-0040: Derive each copy of a generic class from the copy of its base

## Status

Accepted

## Date

2026-11-16

## Context

ADR-0028 makes each copy of a generic class, such as `Box[int]`, a
class of its own with its own layout and methods, whose type object
names the generic class as its base and as its public type. It left out
a generic class with a base other than `object` and a class derived from
a generic class or from one of its copies. GEN-08 asks for both.

In CPython there is one class. `class Box[T](Base):` makes `Box` derive
from `Base`, so every `Box[int]` object is a `Base`.
`class IntBox(Box[int]):` makes `IntBox` derive from `Box`, since
`Box[int]` gives `Box` as the base, and `class Sub[T](Box[T]):` does the
same for `Sub`. `isinstance` holds for every class of the chain.

TyPy lays out an object by its class, and a class derived from another
extends the other's layout and method table (ADR-0021). An `IntBox`
holds the `int` of `Box[int]` in place, so its layout follows that copy,
not the generic class, which has no layout of its own.

## Options considered

1. **Derive from the copy.** The base of a copy is the generic class's
   base, with each copy among the classes the generic class derives from
   made for the copy's type arguments. A class derived from `Box[int]`
   derives from that copy, and a copy of `Sub[T](Box[T])` for `int`
   derives from `Box[int]`.
2. **Derive from the generic class.** A class derived from a copy
   derives from the generic class, and each of its attributes of a type
   parameter is held as an object, so one layout serves every copy. It
   costs an allocation for each such attribute, which ADR-0023 set out to
   avoid.

## Decision

TyPy uses option 1.

- **A base of a generic class.** `class Box[T](Base):` derives the
  generic class from `Base`, and each copy from `Base` too: a copy's
  layout and method table extend `Base`'s, and the checker knows that a
  `Box[int]` is a `Base`. A method of `Box` that overrides one of `Base`
  is checked as any override is, with the type parameters opaque.
- **A copy as a base.** `class IntBox(Box[int]):` derives from the copy
  `Box[int]`: it holds that copy's attributes first and inherits its
  methods. `class Sub[T](Box[T]):` derives the generic class `Sub` from
  the copy of `Box` for `Sub`'s own parameters, and each copy of `Sub`
  from the copy of `Box` for the copy's arguments.
- **Identity.** A class's type object names its base's, and a copy's
  names the generic class's, as ADR-0028 has it, so the chain of an
  `IntBox` runs through `Box[int]` and `Box` to `Base`, and `isinstance`
  with any of them holds, as in CPython.
- **Narrowing.** `isinstance(x, Box)` on a variable whose type is a base
  of `Box` keeps the variable's type: the copy the object belongs to is
  known only when the program runs. Narrowing it to the copies of `Box`
  the program makes is planned for 1.0 (GEN-14), and so are the
  comparison methods of a generic dataclass derived from a dataclass,
  which read the other object's fields after that test.
- **Not yet.** A generic exception class (GEN-12), a generic protocol
  (GEN-13), and a class variable or a default of a generic class typed by
  a type parameter (GEN-11) are planned for 1.0.

## Consequences

- A generic class and the classes derived from it hold every attribute
  in place, and a method call on a copy or on a class derived from it is
  a direct call or a call through its method table, as for any class.
- A class derived from a generic class has its own copies only when it
  is generic itself; `IntBox` is one class.
- The checker makes the copy a base names before it collects the class
  derived from it, so a copy may be made earlier than the first call or
  annotation that needs it.

## Related requirements

GEN-08, GEN-11, GEN-12, GEN-13, GEN-14, CLS-04, ADR-0021, ADR-0023,
ADR-0028.
