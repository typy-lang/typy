# ADR-0034: Type exception args by the raises that create them

## Status

Accepted

## Date

2026-11-15

## Context

EXC-15 asks for `args` of an exception and `code` of a `SystemExit` as
values of their own: stored, passed, indexed and iterated. ADR-0030 left
them to the uses the runtime can answer without a type, such as printing
and comparing, and deferred the rest until TyPy had a type for them.

CPython types them as `tuple[Any, ...]` and `Any`. TyPy has no type of
any value, and TYP-10 keeps one type per value. But a TyPy program is
compiled whole, so every place an exception can be created is known: each
raise and call of an exception class in the program, and each exception
the runtime raises, whose arguments the runtime fixes. Most programs give
an exception class one shape of arguments, often a single message, so
`args` has one tuple type in practice.

Two facts constrain the design. Modules are checked one after another, so
when one module reads `error.args`, a raise in a later module is not
typed yet. And a union that holds tuples is a tag with a slot per member
(ADR-0027), not an object, so `args` is read from the exception's argument
tuple item by item, by its layout's descriptor (ADR-0013).

## Options considered

1. **Infer the types from the raises that create the exception.** `args`
   of a value of exception class `C` has the union of the argument tuples
   of every way the program and the runtime create a `C` or a class
   derived from it. Every value keeps one type, and `args` can be passed
   to a parameter annotated with that type.
2. **One opaque kind of value for exception parts**, whose items the
   runtime reads by their descriptor. Stored, indexed and iterated, but no
   annotation can name it, so it cannot be passed to a typed parameter.
3. **A value of any type for 1.0**, with EXC-15 moved there.

## Decision

TyPy uses option 1.

- **What creates an exception.** A call of an exception class in the
  program gives a tuple of its arguments' types; for a class whose
  `__init__` calls `super().__init__(...)`, the arguments of that call
  (CLS-14). `raise C` without a call gives `tuple[()]`. The runtime's own
  raises are a table: a message, `tuple[str]`, for most classes;
  `KeyError`, the key, of each key type of a dict and item type of a set
  in the program; `OSError` and its subclasses, `(int, str)` and
  `(int, str, str)`; the Unicode errors, their five arguments;
  `StopIteration` from `next()`, `tuple[()]`; and `SystemExit` and
  `AssertionError`, the program's `sys.exit` and `assert` values.
- **The type of `args`.** For a value whose static type is class `C`, the
  union of the tuples of every creation of `C` and of each class derived
  from it, members in a fixed order. `code` of a `SystemExit` is the
  union of `None` for no argument, the argument's type for one, and the
  tuple for several.
- **The order of checking.** A first check of the program records each
  creation, and whether any use of `args` or `code` needs its type. Only
  when one does, the program is checked again with the types the first
  check found, and again while a check finds a creation the one before
  did not, so a raise whose arguments come from `args` settles. A program
  that never reads `args` or `code` as a value is checked once, as
  before.
- **Reading the value.** Lowering reads `args` from the exception's
  argument tuple: it compares the tuple's descriptor with each member of
  the union and reads the items of the matching one, each a word of the
  runtime. An item that is itself a tuple is E0201, planned for 1.0.
  ADR-0030's operations, such as `print(error.args)`, keep their runtime
  calls.

## Consequences

- `args` and `code` are typed values that a program stores, passes,
  indexes, unpacks and iterates, with no new kind of value and no boxing.
- A program that reads them as values is checked two or more times, which
  lengthens its check by the time of each extra pass; others are not.
- A raise added anywhere in the program can change the type of `args`
  elsewhere, as a new member of a union, which may refuse an operation
  that was accepted before, such as an index past a shorter tuple's end.
- Iteration over a union of tuples, needed for `for item in error.args`,
  is added for every union of tuples.
- The runtime's table of its own raises is part of the design; each new
  raise of the runtime adds its row.

## Related requirements

EXC-11, EXC-13, EXC-14, EXC-15, CLS-14, TYP-10, TYP-14, ADR-0013,
ADR-0027, ADR-0030.
