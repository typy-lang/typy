# ADR-0041: Hold a protocol value as an object and call it through a dispatcher

## Status

Accepted

## Date

2026-11-16

## Context

ADR-0023 compiles a parameter whose annotation names a protocol to one
copy of the function for each class it is called with, and decides that
where a protocol is the type of a variable, an attribute, a result or a
container item, its value is the union of the program's types that
satisfy the protocol, and a member call runs by a switch on the type.
GEN-10 asks for those places in 0.5.

Two facts shape how. The classes that satisfy a protocol are known only
when every module is collected: a protocol is often written by the
module that uses it, after the modules that define its classes. And the
checker decides that a value fits a place through `Type::is_assignable_to`,
a test of the two types alone, made in some forty places, including the
conversions of an item of a display and of an argument.

## Options considered

1. **A union of the classes, built at once.** The protocol type becomes
   a `Type::Union` of the classes that satisfy it when an annotation names
   it. Classes of modules checked later would not be members, and a type
   written into a signature cannot grow afterwards.
2. **An object, with the fit known for the whole check and a dispatcher
   built at the end.** A protocol value is an object, as a union of
   objects is (ADR-0027). When a module's classes are collected, and again
   when its `__init__` methods have given its attributes their types, the
   checker notes each class that satisfies each protocol, and the test of
   one class deriving from another takes those notes into account, so
   every conversion that takes an instance of a base takes one of a
   satisfying class. A member call or an attribute read on a protocol
   value calls a dispatcher function for the protocol and member, whose
   body is written once every module is checked: a switch over the
   classes that satisfy the protocol, each arm calling the class's method
   or reading its attribute.
3. **Tables of the protocol's members**, as Go interfaces have, one for
   each class converted to the protocol. A table would have to be built
   at each conversion and carried with the value, which ADR-0023 set aside.

## Decision

TyPy uses option 2.

- **The value.** A protocol type outside a parameter is the protocol's
  instance type, held as an object whose header says what it is; an
  instance of a class that satisfies the protocol converts to it as an
  instance converts to its base.
- **The fit.** The checker notes, in the program and in a table scoped to
  the check, each pair of a class and a protocol it satisfies by the
  rules of GEN-04. The table is set when the check starts and cleared
  when it ends, also after a failure, so no state outlives a check.
- **Members.** `value.name(...)` on a protocol value calls the
  dispatcher `<protocol Protocol.name>`, which takes the protocol's
  method's parameters; `value.name` for an attribute calls a getter
  dispatcher. Its body is an `EachMember` over the satisfying classes,
  the most derived first, each arm converting the arguments to the
  class's method's types and its result to the protocol's.
- **Parameters of special methods.** A parameter of `__init__` or of
  another special method, or of a property, whose annotation names a
  protocol takes protocol values, since such a method has one body and
  cannot be copied for each class.
- **Not yet.** Assigning an attribute through a protocol value, a class
  derived from a protocol, a protocol's properties, static methods,
  class methods, class variables and defaults, a protocol named in its
  own methods' signatures, and a value of a built-in type held as a
  protocol are planned for 1.0 (GEN-17).

## Consequences

- A call on a protocol value costs a test for each satisfying class
  before it, at most, and the call; a parameter keeps the copies of
  ADR-0023, which cost nothing at run time.
- The dispatcher's switch holds every class of the program that
  satisfies the protocol, also those whose instances never become a
  protocol value; code grows with them.
- Lowering needs no table of fits: the checker converts each instance to
  the protocol where it is held, also inside a union.

## Related requirements

GEN-04, GEN-10, GEN-16, GEN-17, TYP-14, ADR-0023, ADR-0027.
