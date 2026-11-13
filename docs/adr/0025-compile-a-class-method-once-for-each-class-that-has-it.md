# ADR-0025: Compile a class method once for each class that has it

## Status

Accepted

## Date

2026-11-13

## Context

CLS-06 asks for `@classmethod`. A class method takes the class it is
called through as its first parameter, `cls`. In CPython `Shape.create()`
passes `Shape` and `Square.create()` passes `Square`, so `cls(...)` makes
an instance of the class the call names, `cls.name` reads that class's
variable, and `cls.other()` calls that class's class method. Alternative
constructors, the most common use, depend on it.

A TyPy value has its type when the program is compiled (TYP-11). An
instance is made from a class known then (ADR-0012), and a class's
`__init__` may take other parameters than its base's.

## Options considered

1. **The class as a value when the program runs.** One body takes `cls` as
   a type object. `cls(...)` would need the class's layout and its
   `__init__` found when the program runs, and the arguments could not be
   checked against a subclass's `__init__`.
2. **Bind `cls` to the class that defines the method.** One body, in which
   `cls` is that class. `Square.create()` would make a `Shape`, unlike
   CPython, unless calls through subclasses are refused.
3. **One copy per class.** Each class that has the class method, its own
   or inherited, gets a copy whose `cls` is that class, checked and
   compiled as an ordinary function. This is how ADR-0023 compiles
   generics.

## Decision

TyPy uses option 3.

- **Copies.** The class that defines a class method, and each class of the
  same module that derives from it and does not define it again, has a
  copy named after the class, such as `Square.create`. Each copy is
  checked with `cls` of the type of its class's type object, so `cls(...)`
  is checked against that class's `__init__`, and an error names the
  class whose copy has it.
- **Calls.** `Class.name(...)` calls the copy of that class, directly.
  Inside a copy, `cls(...)` makes an instance of its class, and
  `cls.name` and `cls.name(...)` read and call through its class.
- **Refused for now.** Calling a class method through an instance, which in
  CPython passes the instance's class, needs dispatch on that class; and a
  class of another module that inherits a class method needs its copy
  checked in the defining module. Both are E0201, planned for 0.5.

## Consequences

- A class method costs code once for each class that has it, and a call
  costs what a call of a function costs.
- A subclass whose `__init__` does not fit the arguments a class method
  passes to `cls(...)` is reported where the copy is checked, rather
  than raising `TypeError` when the program runs.
- Copies for subclasses in other modules wait for the checker to check a
  body in the scope of a module already checked, which ADR-0023's
  generics need too.

## Related requirements

CLS-06, TYP-11, ADR-0012, ADR-0021, ADR-0023.
