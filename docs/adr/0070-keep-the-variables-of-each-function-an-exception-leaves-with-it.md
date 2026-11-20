# ADR-0070: Keep the variables of each function an exception leaves with it

## Status

Accepted

## Date

2026-11-19

## Context

A generator held by a variable closes when its last reference goes
(FUN-11, ADR-0069). In CPython 3.12.16 and 3.15.2, checked on
2026-11-19, an exception's traceback holds the frame of each function the
exception left, and the frame holds that function's variables. A
generator in such a variable stays alive while the exception does: it
closes after the `except` clause that handles the exception ends, or when
a variable that saved the exception lets it go, or, for an exception
nothing catches, after its report. The values a function computes but has
not stored, on CPython's value stack, go at once, and so does the
iterator of a loop the exception leaves.

TyPy's traceback holds the name, file and line of each function, not its
variables, and an error path releases a function's variables as it
leaves (ADR-0013). A generator in a variable then closes before the
`except` clause runs, so its `finally` prints before the handler's
output, where CPython prints it after.

## Options considered

1. **Release the variables at once,** as before. Simple, but the output
   comes in another order than CPython's whenever a generator's closing
   prints.
2. **Keep the variables that may reach a generator with the exception.**
   The error path hands such a variable's reference to the exception,
   which releases it when it is freed.
3. **Keep every frame whole,** as CPython does. A traceback would need
   the layout of every function's variables, for an effect only a
   generator's closing can show.

## Decision

Option 2.

- **In a program with a generator function,** the error path of a
  function hands each of its variables whose value is an object other
  than a `str`, `bytes` or a type, or an optional one, to the
  exception's traceback, through `typy_rt_exception_keep`. The
  variables of a generator's body go the same way. A program without
  one releases its variables as before.
- **The arguments of a failed call** of the program's code that the
  caller owns go with the exception too, since they were the callee's
  variables in CPython. Hidden slots and other temporaries are released
  at once, as CPython's value stack is.
- **The exception releases what it keeps** when it is freed, the
  innermost function's first, so a generator among them closes when the
  exception goes. An uncaught exception goes after its report, and
  `main` then closes the generators it held.
- **The fixed part of an exception** grows by the list it keeps, from 80
  to 96 bytes, which the compiler's `EXCEPTION_PREFIX` follows.

## Consequences

- A generator in a variable of a function an exception left closes
  where CPython closes it, also when the exception is saved.
- An exception holds the memory of those variables until it goes, as
  CPython's does; a deep recursion that ends in `RecursionError` keeps
  the lists of every frame until the error is handled.
- The error path of a function in a program with generators makes one
  runtime call more for each variable it keeps.

## Related requirements

FUN-11, EXC-04, MEM-01, ADR-0013, ADR-0069.
