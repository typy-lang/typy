# ADR-0013: Make exceptions objects of exception classes

## Status

Accepted

## Date

2026-11-10

## Context

ADR-0004 compiles exceptions as error values returned to the caller. In
0.2 nothing catches them: every error path releases what the function owns
and returns the exception, until it reaches `main` and is reported. Each
exception carries a fixed kind from a closed list, a message built when it
is raised, a traceback and a cause.

Release 0.3 adds `raise`, `try` with `except`, `else` and `finally`,
`assert`, exception classes written by programs and exception chaining
(SMT-04, SMT-06, CST-04, CLS-03, EXC-01 to EXC-04). These need four things
the 0.2 form lacks:

- `except LookupError` catches a `KeyError`, so a handler matches on base
  classes, and a program's own class `class AppError(ValueError)` sits in
  the same hierarchy as the runtime's classes.
- `str(e)` follows the arguments the exception was built with. CPython
  computes it when asked: `KeyError('a')` shows `'a'` and a list argument
  shows its current items.
- CPython sets `__context__` when an exception is raised while another is
  being handled, also when the raise is in a function a handler calls.
- `finally` runs on every way out of a `try`.

ADR-0012 already gives every type a type object with a name, a base and
slots, and its instance layout of header, class and fields. General
inheritance with overriding is CLS-04, planned for 0.4.

## Options considered

1. **Keep the closed list of kinds** and add user classes as a separate
   case. Matching on bases would need a table of the runtime's hierarchy
   in the compiler, and a user class could not derive from `ValueError`.
2. **Exceptions as objects of exception classes.** The kind becomes a
   pointer to a type object. The runtime defines the built-in classes with
   CPython's bases; a program's exception class is a type object whose base
   is one of them. Matching walks the chain of bases.
3. **Exceptions as ordinary class instances** with no exception part. The
   traceback, cause, context and arguments would have to be fields every
   exception class declares, and the runtime could not raise without
   knowing a program's classes.

For the implicit context, two forms were weighed: set it when an exception
leaves a handler in the same function, or keep the handled exception as
runtime state and set the context when an exception is raised, as CPython
does. The first misses an exception raised and caught inside a function
that a handler calls.

For `finally`, two forms were weighed: compile its body once for each way
out of the `try`, or compile it once with a slot that records the way out
and a branch on it at the end. CPython 3.9 and later copy the body for the
normal and the exceptional path.

## Decision

TyPy uses option 2.

- An exception object is the object header, the class, the arguments as a
  tuple, the traceback, the cause, the context and the flag that `from
  None` sets. A class's own fields follow, so a subclass adds fields after
  those of its base and a field keeps its offset in every subclass.
- The runtime defines `BaseException`, `Exception` and the classes of
  EXC-02 and EXC-10 as static type objects with CPython's bases. A program
  names them as the bases of its own exception classes.
- `except C` matches when `C` is the exception's class or one of its bases.
  The runtime walks the chain of bases, which costs a few compares on the
  error path and nothing on the normal path (EXC-03).
- `str(e)` and `repr(e)` are computed from the arguments when asked: no
  argument gives an empty `str`, one gives its `str`, more give the `repr`
  of the tuple, and `KeyError` with one argument gives its `repr`. The type
  object gains a `str` slot, beside `repr`, so a class that defines
  `__str__` or `__repr__` is called through its type object whatever the
  static type of the variable that holds it (EXC-08, CLS-13).
- A program's exception class may redefine `__init__`, `__str__` and
  `__repr__`. Redefining any other method of a base is rejected until 0.4
  brings checked overriding (CLS-04, CLS-13). `super().__init__(...)` is
  accepted in an exception class's `__init__` and sets the arguments
  (CLS-14).
- The runtime keeps the exception being handled. Entering an `except`
  clause pushes it, and every way out of the clause pops it. An exception
  raised while one is handled takes it as its context, unless that would
  form a cycle, as CPython does (EXC-04).
- `finally` is compiled once for each way out of the `try` that the code
  has: falling through, `return`, `break`, `continue` and an exception. A
  `return`, `break` or `continue` that would leave the `finally` block is
  rejected (EXC-06), as PEP 765 makes CPython 3.14 warn about them.
- Inside a `try`, the error path of a call goes to the code that matches
  the handlers. An exception that no handler matches continues on the error
  path of the enclosing `try`, or returns to the caller.
- A bare `raise` re-raises the exception being handled and is accepted only
  inside an `except` clause (EXC-07).

## Consequences

- Built-in and program exception classes share one hierarchy, so a handler
  for `LookupError` or `Exception` catches both, and 0.4 inheritance reuses
  the same layout of base fields followed by own fields.
- A handler costs nothing until an exception arrives. The normal path
  still pays one compare and branch per call that may raise (ADR-0004).
- The handled exception is one piece of mutable runtime state. TyPy
  programs have one thread, so it needs no lock; threads would have to keep
  one per thread.
- Copying `finally` for each way out adds code size for each copy. Rejecting
  `return`, `break` and `continue` out of `finally` removes the paths where
  a pending exception is dropped.
- `__str__` and `__repr__` called through slots work for any exception
  class. Other methods called on a variable whose static type is a base
  call the base's method, which is correct only because overriding them is
  rejected until 0.4.
- `e.args` needs a static type for a tuple of any values, which TyPy has
  not got. It is planned for 0.4 (EXC-11).

## Related requirements

EXC-01 to EXC-04, EXC-06 to EXC-12, SMT-04, SMT-06, CST-04, CLS-03,
CLS-13, CLS-14, STD-05, NFR-01, NFR-02.
