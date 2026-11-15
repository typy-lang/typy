# ADR-0036: Call the function the last-run import bound

## Status

Accepted

## Date

2026-11-15

## Context

TyPy resolves a call of a function or a class that an import binds by
name when compiling: `from first import run` makes `run()` a direct call
of `first.run`. That holds while one import binds the name and it runs
before every call, which E0306 enforces for an import outside any block
(STD-06). It fails in two cases CPython allows:

- **Several imports bind the name to different functions or classes**,
  eager or lazy: `from first import run` then `from second import run`,
  or one in each branch of an `if`. CPython calls the one the import that
  ran last bound. TyPy called the last one in the source everywhere, and
  now refuses the name with E0202.
- **An import inside a block binds the name**, such as
  `if sys.version_info >= (3, 13): from fast import parse`. CPython
  raises `NameError` when no such import has run; TyPy called the
  function anyway.

MOD-12 asks for the lazy form of the first case, and it was widened to
eager imports and to imports inside blocks, since one mechanism serves
all three.

## Options considered

1. **A hidden binding variable per name.** Each import statement that
   binds the name stores its own number in an `int` module variable, 0
   while none has run. A call tests the number and calls the function or
   class of that import, running its lazy import first when it is lazy,
   and raises `NameError` for 0. The test is a load and a comparison per
   import, only for names that need it.
2. **Function values.** Bind the name to a value that holds the function,
   and call through it. Needs function values (FUN-05), planned for 1.0,
   and a way to type a call of one of several signatures.
3. **Keep refusing** both cases until 1.0.

## Decision

TyPy uses option 1.

- **Which names.** A module-level name that imports bind to two or more
  different functions, classes or functions of a built-in module, or that
  an import inside a block binds, gets a module variable `<bound name>`,
  whose `NameError` names `name`. A name one import outside any block
  binds keeps its direct calls.
- **The binding.** Each import statement that binds the name stores its
  number, from 1 in source order, when it runs; a lazy import stores it
  when its statement runs, and its own function runs the import at the
  call.
- **A call** is a chain of `if` tests on the number, one arm per binding
  import, each checked as a call of that import's function or class, and
  a last arm that raises `NameError`. The result is the union of the
  arms' results.
- **A read of the name as a value** of one function or class is guarded
  the same way. A name bound to several is E0201 when read as a value,
  named in an annotation or imported from another module, planned for
  1.0 (MOD-14), since its value and its type depend on which import ran.

## Consequences

- Version and platform conditional imports of functions and classes
  compile with CPython's meaning, as do two lazy imports of one name.
- A call of such a name costs a load and a comparison for each binding
  import before it; names bound once outside a block cost nothing.
- The arguments of such a call are checked once for each binding, so a
  call that one of the functions cannot take is refused, though the
  program might never run that import.

## Related requirements

MOD-06, MOD-11, MOD-12, MOD-14, STD-04, STD-06, TYP-12.
