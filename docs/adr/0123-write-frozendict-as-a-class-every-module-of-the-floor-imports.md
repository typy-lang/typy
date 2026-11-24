# ADR-0123: Write frozendict as a class every module of the floor imports

## Status

Accepted

## Date

2026-11-24

## Context

Python 3.15 adds the builtin `frozendict` (PEP 814), an immutable mapping
that hashes when its values do (DAT-18). ADR-0119 gives it a home in the
`typy-stdlib` crate, which every module of a program shares, so a
`frozendict[str, int]` of one module is the type of another's.

A class written in typed Python gives most of it as the program's own
classes are compiled: `len`, `in`, iteration, `fd[key]`, `|`, `==`, the
text and the hash, which is CPython's hash of the frozenset of the items
once `hash` exists (LIB-34, ADR-0122). Three parts are not a class's:

- A module names `frozendict` without importing it, as it names `dict`.
- `frozendict(...)` takes what `dict(...)` takes: a mapping, pairs,
  keywords, or nothing; a typed `__init__` takes one of them.
- Code written for dicts reads a `frozendict` as a mapping: `dict(fd)`,
  `d.update(fd)`, `{**fd}`, `f(**fd)` and `d | fd`. `get` takes a default
  of the value type, which needs `@overload` in a generic class, planned
  for 0.13 (TYP-35).

## Options considered

1. **A built-in type of the runtime,** as `dict` is, with its own methods
   in the checker, lowering and runtime.
2. **A generic class written in typed Python,** in the `typy-stdlib`
   module `_frozendict`, which a module imports when it names
   `frozendict`, with the three parts above given by the checker.

## Decision

Option 2.

- **The class.** `_frozendict.frozendict[K, V]` holds its items in a dict
  field `_items`, copied when it is made. It is Python that CPython runs,
  and gave CPython 3.15.2's results on 30,000 checks of its text, views,
  `|`, equality, `get`, `copy` and hash.
- **The import.** After a module of the Python floor 3.15 or later is
  parsed, a module whose text holds the word `frozendict` and whose module
  level binds no such name gets `from _frozendict import frozendict` at
  its start, after its docstring. A word in a comment or a string adds an
  import that changes nothing the module does. Lowering writes the class
  without a module, as `<class 'frozendict'>`.
- **Calls.** `frozendict(arguments)` is checked as
  `frozendict(dict(arguments))`. Where a `frozendict[K, V]` is expected,
  the call is of that copy, so `frozendict()` takes its types.
- **Reading as a dict.** Where a loop, `dict(...)`, `dict.update`, `**`
  in a display or a call, or `|` and `|=` after a dict read a mapping, a
  `frozendict` reads as its `_items`. `get`, `keys`, `values` and `items`
  are the dict's methods of `_items`, with their signatures.
- **Below the floor.** `frozendict` is E0425, naming the floor 3.15.

## Consequences

- A `frozendict` behaves as CPython 3.15's in every way a typed program
  sees, its hash included; `fd[key] = value` and a method that would
  change it are refused as for any class without them.
- `frozendict(fd)` makes a new object where CPython 3.15 returns `fd`
  itself; only `is` shows the difference.
- A program that names `_frozendict` itself reaches the class, as it
  reaches any private name of a module.

## Related requirements

DAT-18, LIB-34, LIB-27, TYP-35, ADR-0119, ADR-0122, ADR-0089.
