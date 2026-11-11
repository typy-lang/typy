# ADR-0015: Compile modules into one program, with module objects

## Status

Accepted

## Date

2026-11-11

## Context

Releases 0.1 and 0.2 compile one module, the main file, which runs as
`__main__` (STD-06). Its module-level names are locals of the module's
code, so a function cannot read them: that is E0201, planned with SMT-08.

Release 0.3 builds a program from several files (MOD-01), with packages
and relative imports (MOD-02), each module initialised once in CPython's
order (MOD-03), circular imports rejected with the cycle shown (MOD-05),
`import` and `from ... import` (SMT-07) and `global` (SMT-08). TYP-15 and
CST-10 need `Iterator`, `Iterable` and `TracebackType` from `typing`,
`collections.abc` and `types`. Lazy imports (MOD-06 to MOD-09) follow in
the next step of the release and build on this one.

CPython defines the behaviour to keep:

- `import m` runs `m`'s code the first time and binds the name `m` to a
  module object; `m.x` reads the module's variable as it is now.
- `from m import x` binds `x` to the value `m.x` has at that moment; a
  later assignment in `m` does not change it.
- A module whose code raised is removed from `sys.modules`, so the next
  import runs its code again.
- A function reading a module variable that is not bound yet raises
  `NameError: name 'x' is not defined`; `m.x` raises `AttributeError:
  module 'm' has no attribute 'x'`; `from m import x` raises
  `ImportError: cannot import name 'x' from 'm' (path)`.
- `import a.b` initialises `a`, then `a.b`, binds `a`, and sets `b` as an
  attribute of `a` once `a.b` is initialised.
- A module object is written `<module 'm' from '/abs/path/m.py'>`.

## Options considered

For where module variables live:

1. **Keep them as locals of the module's code, and pass them to functions.**
   Every function would take hidden parameters, and another module could
   not reach them.
2. **Static storage for each module variable**, one slot per name of its
   checked type, which module code, functions and other modules read and
   write. A name a function or another module could read before it is
   bound gets a flag that says whether it is bound.

For which imports may form a cycle:

1. **Reject every cycle of `import` statements**, wherever they stand.
   Valid programs that import inside a function to break a cycle are
   rejected.
2. **Reject a cycle of imports at module level only.** An import inside a
   function runs later, when the program calls it, and may find a module
   still initialising, as in CPython; the bound flags make that safe.

For modules as values, the user chose objects with attributes known at
compile time over names that only `m.x` may follow. For a module's text,
the user chose the path from the project root over the absolute path,
which would make the build depend on its directory (NFR-06).

## Decision

TyPy uses static storage for module variables and rejects module-level
cycles only.

- **Loading.** `Project::load` reads the main file and every module it
  imports, at any depth and inside functions too, in the order of the
  qualified names. A module `a.b` is `a/b.py` or the package
  `a/b/__init__.py`, found first under the main file's directory, then
  among the standard library modules embedded in `typy`, so a user module
  hides a standard library module of the same name, as CPython's path
  order does. A directory without `__init__.py` is not a package.
  Importing the main file by its own name loads a second module from it,
  as CPython does.
- **One program.** Every module compiles into the one executable. Each
  module's code is a function of its own, run once by its first import.
- **Module variables.** Each name a module binds at module level is a
  module variable in static storage, of the one type the checker gives
  it. Module code reads and writes it as it reads a local, with definite
  assignment. A function reads it, and assigns it only after `global`
  (SMT-08). A name a function or another module reads gets a bound flag;
  a read while it is unbound raises CPython's exception for that read.
  The variables of a module are released only when the program ends.
- **Initialisation.** A module is not started, running or done. An import
  of a module that is not started runs its code, which makes it running,
  and done when the code returns. Code that raises leaves the module not
  started and raises on, so the next import runs it again. Lazy imports
  use the same states (MOD-06).
- **Cycles.** The imports at the module level of each module form a
  graph; a cycle in it is E0422, with each module and import of the cycle
  shown (MOD-05). An import inside a function or a method adds no edge.
- **Module objects.** `import m` binds an immortal module object of type
  `module`, typed `Type::Module(m)`, whose attributes are its module
  variables and, for a package, its initialised submodules. `m.x` reads
  the variable, `m.x = v` assigns it with its type, `n = m` binds the same
  object, and `is` compares modules. The text is CPython's with the path
  from the project root, `<module 'a.b' from 'a/b.py'>`, the one known
  difference (OI-20). A value annotated `types.ModuleType` is a module
  whose attributes are not known, so reading one through it is E0412.
- **Imported names.** `from m import x` initialises `m`, then copies the
  value `x` has into the importer's own variable. `from a import b`
  imports the submodule `a.b` when `a` binds no `b`. `import a.b as c`
  binds `c` to `a.b`. A relative import resolves against the importing
  module's package (MOD-02).
- **Names for annotations.** `typing` and `collections.abc` give
  `Iterator` and `Iterable`, and `types` gives `TracebackType` and
  `ModuleType`, for annotations only in 0.3; their use as a value is
  E0201, planned for 0.4 with `typing` (ADR-0007).
- **Build cache.** The key of `typy run`'s cache covers every module's
  source and the compiler's build, so no change to any file or to the
  compiler reuses an old binary.

## Consequences

- A function can read the variables of its module, and with `global`
  assign them, at the cost of a test of the bound flag where the checker
  cannot see that the variable is bound.
- `from m import x` and `m.x` give CPython's results when `m` later
  assigns `x`, because one copies the value and the other reads the
  variable.
- A program whose modules import each other at module level is rejected,
  although CPython runs some such programs; an import moved into a
  function, a common way to break a cycle, is accepted.
- A module's text differs from CPython's in its path only, so the output
  of a program that prints a module depends on the project root and not
  on the machine.
- `typing`, `collections.abc` and `types` exist only for annotations
  until 0.4.

## Related requirements

SMT-07, SMT-08, MOD-01, MOD-02, MOD-03, MOD-05, MOD-10, TYP-15, CST-10,
STD-06, NFR-06, ADR-0007, ADR-0008, ADR-0012.
