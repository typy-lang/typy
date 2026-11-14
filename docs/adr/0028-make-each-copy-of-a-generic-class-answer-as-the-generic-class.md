# ADR-0028: Make each copy of a generic class answer as the generic class

## Status

Accepted

## Date

2026-11-14

## Context

ADR-0023 compiles a generic function or class to one copy for each set
of type arguments the program reaches. For a generic function a copy is
an ordinary function. A generic class, `class Box[T]:`, has instances,
and an instance of `Box[int]` holds an `int` word where one of
`Box[str]` holds an address: the copies lay out their attributes
differently, so each needs a type object of its own, which records the
layout the runtime releases an instance by.

In CPython there is one class. `type(Box(1))` is `Box`,
`isinstance(Box(1), Box)` holds, `type(x).__name__` is `"Box"`, and the
default `repr` of an instance names `Box`. `Box[int]` is a value of its
own at run time, an alias whose call makes a `Box`.

## Options considered

1. **One class, every attribute boxed.** The copies share a type object
   and every attribute of a type parameter's type is held as an object.
   An `int` attribute costs an allocation, which ADR-0022 and ADR-0023
   set out to avoid.
2. **Independent copies.** Each copy is a class named `Box`, unrelated
   to the others. `isinstance(x, Box)` and `type(x) is Box` give `False`
   where CPython gives `True`, so the program means something else.
3. **Copies that answer as the generic class.** Each copy is a class of
   its own, with its own layout and methods, whose type object names the
   generic class as its base and as its public type. The generic class
   has a type object of its own, with no attributes, and no instances.

## Decision

TyPy uses option 3.

- **The generic class** is checked as a generic function is: its methods
  once each, with its type parameters opaque. It has a type object, named
  as the class is, which `isinstance` and `type()` compare with, and no
  instance is ever made of it.
- **A copy** is made for each set of type arguments the program reaches:
  by a call, `Box(1)`, whose type arguments come from `__init__`'s
  arguments as a generic function's do, by `Box[int](1)`, or by an
  annotation, `Box[int]`. Its attributes and its methods' signatures are
  the generic class's with the type parameters replaced, and its methods
  are checked as copies are.
- **Identity.** A copy's type object names the generic class's as its
  base, so `isinstance(x, Box)` holds, and as its public type, which
  `type()` gives, so `type(x) is Box`. Its name and its module are the
  generic class's, so `repr` and `__name__` are CPython's. The checker
  knows statically that an instance of a copy is an instance of its
  generic class.
- **Not yet.** A generic class with a base other than `object`, a class
  derived from a generic class or from one of its copies, and a generic
  dataclass are refused until a later release names them.

## Consequences

- An instance of `Box[int]` holds its `int` in place, and a method call
  on it is a direct call of the copy's method.
- The type object gains one field, the public type, which is null for
  every class but a copy; `type()` of an object reads it.
- Each copy adds a type object and a copy of each method to the program;
  code grows with the number of copies, as ADR-0023 accepts.
- `Box[int]` used as a value other than in a call, such as stored in a
  variable, is refused, as TyPy refuses other generic aliases.

## Related requirements

GEN-01, GEN-02, GEN-03, CLS-01, CLS-09, CLS-12, ADR-0012, ADR-0021,
ADR-0023.
