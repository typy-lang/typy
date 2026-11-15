# ADR-0023: Compile generics and protocol parameters to one copy per concrete type

## Status

Accepted

## Date

2026-11-13

## Context

GEN-01 asks for generic functions and classes written with PEP 695 type
parameter lists, such as `def first[T](items: list[T]) -> T` and
`class Box[T]:`, and refuses `TypeVar` and `Generic`. GEN-02 adds a
bound, `[T: Shape]`, and constraints, `[T: (int, str)]`. GEN-03 asks for
one copy per concrete type, with no type information at run time.
GEN-04 asks for classes derived from `typing.Protocol`, satisfied by
any class with matching members, and GEN-05 refuses `isinstance` with a
protocol.

A value's representation depends on its type: an `int` is a word, a
`float` a double, a tuple its items in place (ADR-0022). A single
compiled body for every `T` would need every value boxed, or a
description of `T` passed at run time, which GEN-03 rules out.

A protocol type may appear in two kinds of place. As a parameter's type,
each call passes a value whose class the caller knows. As the type of a
variable, an attribute, a container item or a result, one place may hold
values of different classes over time, as a `list[Shape]` holding a
`Circle` and a `Square` does.

## Options considered

1. **One copy per concrete type.** A generic function, or a function
   with a protocol parameter, compiles once for each set of types it is
   called with. Calls are direct and can be inlined; code grows with the
   number of copies.
2. **Dispatch at run time.** One compiled body takes its protocol
   parameter with a table of the members it uses, as Go interfaces and
   Rust's `dyn` do. Code stays small; each member call is indirect.

## Decision

TyPy uses option 1.

- **Checking.** A generic body is checked once, with each type parameter
  an opaque type that offers what its bound offers; a constrained
  parameter is checked once per constraint. An error in a body is
  reported once, at the definition.
- **Copies.** Type arguments are inferred from a call's arguments, or
  given, as in `Box[int]()`. Each set of type arguments the program
  reaches makes a copy, named after its type arguments, lowered and
  generated as an ordinary function or class. Only copies reached from
  the entry are made.
- **Recursion.** A copy that needs a copy of itself with a larger type,
  such as `f[T]` calling `f[list[T]]`, would never end; past 64 nested
  copies the program is refused with an error that names the chain.
- **Protocols.** A class satisfies a protocol when it has each of the
  protocol's methods and attributes with a compatible type, checked when
  the program is compiled; a missing or mismatched member is an error
  that names it. A parameter whose type is a protocol is a type
  parameter bounded by the protocol: `def total(shape: Shape)` compiles
  as `def total[S: Shape](shape: S)`.
- **Protocol types that hold several classes.** Where a protocol is the
  type of a variable, an attribute, a container item or a result, its
  value is the union (ADR-0022) of the program's types that satisfy the
  protocol, and a member call runs by a switch on the type.
- **Refused.** `TypeVar`, `Generic` and `isinstance` with a protocol are
  errors, with GEN-01 and GEN-05.

## Consequences

- A generic call costs what a call written for its concrete types
  costs; the price is code size and compile time, which grow with the
  number of copies.
- Two modules that make the same copy share one, as tuple descriptors
  are shared: the copy is a link-once symbol named after its types.
- Under separate compilation (ADR-0016) a module that uses another's
  generic needs its body, so the interface carries the source of each
  generic function and class, and the protocol unions are built at link
  time.
- A protocol stored in a container dispatches by a switch over the types
  that satisfy it, which grows with them.

## Related requirements

GEN-01, GEN-02, GEN-03, GEN-04, GEN-05, TYP-14, ADR-0016, ADR-0021,
ADR-0022.
