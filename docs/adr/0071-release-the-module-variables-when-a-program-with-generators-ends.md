# ADR-0071: Release the module variables when a program with generators ends

## Status

Accepted

## Date

2026-11-19

## Context

A generator held by a module variable is still alive when the program
ends. CPython 3.12.16 and 3.15.2, checked on 2026-11-19, clear the
modules at exit, and such a generator closes then, after the program's
last output and after the report of an uncaught exception. The variables
of `__main__` go first, in the order the program first bound them, then
those of the other modules. Between the other modules the order follows
CPython's module objects and the reference cycles among them: with
`main` importing `helper` and then `other`, and `other` importing
`helper`, `other` goes before `helper`, but a module imported inside
another one's code can go in the middle of that module's variables.

TyPy keeps module variables in globals and, until 0.9, never released
them: freeing an object ran no code, so nothing could show it, and the
memory goes with the process.

## Options considered

1. **Release nothing at exit.** A generator in a module variable never
   runs its `finally`, where CPython runs it.
2. **Release the module variables at exit in a fixed order:** those of
   `__main__` first, in the order its code binds them, then each other
   module's, the last to run first.
3. **Release them in the order the program first bound them at run
   time,** recording each first binding. Every assignment to a module
   variable would test and record, for a difference that shows only
   when a module first binds its variables out of their written order.

## Decision

Option 2.

- **The function `__typy_finish`,** which the lowering adds to every
  program and the runtime's `main` calls at the end, after the report
  of an uncaught exception and the closing of the generators it kept
  (ADR-0070), releases the module variables of a program with a
  generator function. A program without one has nothing to close, so
  the function only returns.
- **The order** is `__main__`'s variables in the order its code first
  binds them as written, then each other module's, in the reverse of the
  order the modules' code runs in, which puts a module that imports
  another before it.
- **Each variable is unbound** as it is released, so code a closing runs
  that reads it raises `NameError`, which the closing reports as
  ignored, where CPython's code sees the name cleared.

## Consequences

- A generator still suspended in a module variable runs its `finally`
  blocks at the end, as in CPython, after everything else the program
  printed.
- A module that binds a variable conditionally before another, or a
  module imported inside another module's code, may close its generators
  in another order than CPython; a test does not print the closing of
  two such generators.
- A program with generators frees its module variables before it exits,
  which takes time in proportion to what they hold, as in CPython.

## Related requirements

FUN-11, MOD-03, NFR-08, ADR-0069, ADR-0070.
