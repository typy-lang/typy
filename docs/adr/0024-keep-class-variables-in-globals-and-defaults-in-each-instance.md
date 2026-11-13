# ADR-0024: Keep class variables in globals and defaults in each instance

## Status

Accepted

## Date

2026-11-13

## Context

CLS-11 asks for class variables, given a value in the class body. Python
gives a value in a class body two meanings, and type checkers follow
PEP 526 in telling them apart:

- `count = 0` and `count: ClassVar[int] = 0` declare a class variable:
  one value that the class holds and every instance reads through the
  class.
- `size: int = 0` declares an instance attribute whose default is the
  class's value: an instance that never assigns `size` reads the class's
  value, and one that assigns it has its own. `@dataclass` (CLS-05) reads
  these defaults.

In CPython both are attributes of the class object, and an instance reads
the class's attribute when it has none of its own. A TyPy instance has a
fixed layout (ADR-0012), so an attribute is either in every instance or
in none.

## Options considered

1. **CPython's lookup at run time.** Each read of an attribute looks in
   the instance, then in the class. This needs a flag per attribute in
   each instance and a branch on every read.
2. **A class variable in static storage, a default copied into each
   instance.** A class variable is a module variable of the class's
   module; an annotated default is copied into each new instance before
   `__init__` runs. Writes that would make an instance and the class
   disagree are refused.

## Decision

TyPy uses option 2.

- **Class variables.** `name = value` and `name: ClassVar[T] = value` in
  a class body declare a class variable, held as a module variable of
  the class's module and given its value where the `class` statement
  runs, as CPython runs the class body. `Class.name` and `self.name` read
  it, and `Class.name = value` assigns it, in any function. Assigning it
  through an instance, which in CPython gives that instance an attribute
  of its own, is refused.
- **Annotated defaults.** `name: T = value` declares an instance
  attribute with a default. The default is held as a class variable, and
  each new instance starts with that value, the same object, before
  `__init__` runs, so a mutable default is shared as in CPython.
  `Class.name` reads the default; assigning it is refused, since in
  CPython instances that never assigned the attribute would read the new
  value.
- **Inheritance.** A subclass reads its bases' class variables and starts
  its instances with its bases' defaults (ADR-0021).

## Consequences

- Reading a class variable costs a load from static storage; reading an
  attribute with a default costs what any attribute costs.
- An instance is as large as before: a default takes the place its
  attribute takes anyway.
- Programs that give one instance its own value of a class variable, a
  pattern type checkers also flag, are refused with a message that names
  `Class.name`.
- Separate compilation (ADR-0016) exports class variables as it exports
  module variables.

## Related requirements

CLS-05, CLS-11, ADR-0012, ADR-0015, ADR-0016, ADR-0021.
