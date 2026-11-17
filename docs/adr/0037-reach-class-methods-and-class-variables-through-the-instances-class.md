# ADR-0037: Reach class methods and class variables through the instance's class

## Status

Accepted

## Date

2026-11-15

## Context

ADR-0025 compiles a class method once for each class of the defining
module that has it, and calls the copy of the class it is called through.
It left two forms of CPython's to a later release, and ADR-0024 left a
third:

- **A class method called through an instance.** CPython passes the
  instance's class as `cls`, which may be any class derived from the
  instance's static type, including one of a module checked later.
- **A class method inherited by a class of another module.** Its copy
  must be checked in the scope of the module that defines the method,
  whose names its body reads, while the class that needs it is checked in
  its own module.
- **A class variable assigned through a subclass.** In CPython,
  `Square.sides = 4` gives `Square` a variable of its own; until then,
  `Square.sides` reads its base's. A read through an instance reads the
  variable of the instance's class.

CLS-25 asks for all three in 0.5. MOD-13 added what the second needs: a
body of one module can be checked once every module is, in that module's
names.

## Options considered

1. **Copies, bridges in the method tables, and holders read down the
   chain.** Every class that has a class method gets a copy, also in
   another module. Each copy gets a bridge, an instance method of the same
   name that calls the copy with its class, so a call through an instance
   goes through the method table to the bridge of the instance's class. A
   class variable that some module assigns through a subclass gets a
   holder for each class, read down the chain of bases to the first
   holder that is bound.
2. **One class method that takes its class at run time.** `cls(...)`
   would then need a constructor in each type object, chosen when the
   program runs, and every use of `cls` would lose its static class, which
   ADR-0025 chose copies to keep.
3. **Keep refusing** the three forms until 1.0.

## Decision

TyPy uses option 1.

- **Copies in other modules.** The class method statements of each class
  and the names of each checked module are kept until every module is
  checked. A class of another module that inherits a class method gets
  its copy, its signature read in the names of the method's module, and
  its body checked once every module is, in those names.
- **Calls through an instance.** Each copy has a bridge: a method of the
  class method's name that takes the instance and the copy's other
  parameters, and returns the copy's call with the bridge's own class. A
  call through an instance calls the bridge of the instance's static
  class with the method table, so the bridge of its own class runs.
- **Class variables of a subclass.** Before any module is checked, every
  module is read for `Class.name = value` of a class that only inherits
  `name`. Each class of that chain then has a holder of its own, bound
  only once assigned; a read through a class reads the first bound holder
  from that class up, and a read through an instance does so from the
  instance's class, through a method of the method table. Other class
  variables keep ADR-0024's single holder and direct reads.

## Consequences

- A class method costs a bridge for each copy, and a call through an
  instance costs a call through the method table and the call of the
  copy.
- A class variable assigned through a subclass anywhere in the program
  costs its reads a test of a flag for each class up the chain; others
  cost nothing.
- Checking a program whose classes inherit class methods across modules
  checks those copies after every module.

## Related requirements

CLS-06, CLS-11, CLS-25, MOD-13, ADR-0021, ADR-0024, ADR-0025.
