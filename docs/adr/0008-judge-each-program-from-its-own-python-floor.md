# ADR-0008: Judge each program by CPython from its own Python floor

## Status

Accepted

## Date

2026-11-06

## Context

ADR-0002 made TyPy a strict subset of standard Python: TyPy accepts only
source that CPython 3.12 to 3.15 all parse, and an accepted program runs
alike under TyPy and each of those versions.

The project owner wants Python's explicit lazy imports. PEP 810, Final and
part of Python 3.15, adds two forms:

- The soft keyword `lazy`, as in `lazy import json` and
  `lazy from json import dumps`, allowed only at module level. CPython 3.12
  to 3.14 reject it as a syntax error.
- A module-level list `__lazy_modules__`, which makes the named imports lazy
  on 3.15. The PEP says "On earlier versions the declaration is ignored,
  leaving imports eager."

Under ADR-0002 the `lazy` keyword could not be accepted until 3.15 is the
oldest supported version, after 3.14 reaches end of support in 2030-10. The
same would hold for any later syntax. Python projects already state the
oldest version they support, in `requires-python`, and use newer syntax
when they raise it.

The pinned parser, `ruff_python_parser` 0.0.20, parses `lazy import`, marks
the import with `is_lazy`, and reports it as unsupported syntax when the
target version is below 3.15 (ADR-0005).

## Options considered

1. **One floor for every program, 3.12.** STD-01 stays as written. Only
   `__lazy_modules__` is available until 2030-10, and the same wait applies
   to every later syntax addition.
2. **A Python floor per program.** A program declares the oldest CPython it
   supports, 3.12 by default. TyPy parses it with that version as the
   target, and judges it by the oracle versions from that floor up. A
   program with floor 3.15 can use `lazy import` now, and is compared with
   CPython 3.15 only.
3. **Accept 3.15 syntax for every program.** No configuration. But a
   program accepted by TyPy could then fail to parse under CPython 3.12,
   which breaks the standard-Python rule for every user who has not chosen
   3.15.

## Decision

TyPy uses option 2. This record supersedes ADR-0002; everything ADR-0002
decided stands, with the oracle range now set per program.

- Each program has a Python floor: one of the oracle versions, 3.12 by
  default, set with `python-floor` in `typy.toml` (CLI-12).
- TyPy accepts only source that every oracle version from the floor up
  parses (STD-01). The parser's target version is the floor.
- An accepted program runs alike under TyPy and every oracle version from
  its floor up, on every input where those versions finish without an
  uncaught exception (STD-02, STD-05).
- Behaviour that Python leaves to the implementation is not covered
  (STD-03), and a feature that TyPy cannot compile with CPython's meaning is
  rejected with a diagnostic that names the rule (STD-04).
- TyPy 1.0 covers the typeable part of the Python 3.10 language, with PEP
  695 generics. Features from later versions are accepted when the program's
  floor includes them and the specification lists them, as it does for lazy
  imports (MOD-06 to MOD-09).
- Lazy imports follow the Python 3.15 semantics of PEP 810 in TyPy, for both
  the `lazy` keyword and `__lazy_modules__`.

## Consequences

- Programs can use lazy imports from TyPy 0.3, the release that brings
  imports, without waiting for 2030.
- A program with a higher floor is checked against fewer CPython versions.
  The conformance suite runs each test under the versions from its floor up.
- With `__lazy_modules__` and a floor below 3.15, the oracle versions
  differ: 3.12 to 3.14 import eagerly and 3.15 lazily. A test whose output
  depends on when a module initialises is then invalid (STD-03), and TyPy
  follows the 3.15 behaviour.
- CPython's process-wide switches for lazy imports, `-X lazy_imports`,
  `PYTHON_LAZY_IMPORTS`, `sys.set_lazy_imports()` and
  `sys.set_lazy_imports_filter()`, are not provided, because each one changes
  how a program behaves from outside its source. A program that calls them
  is rejected.
- Rules still cite the Language Reference for 3.12. A feature from a later
  version cites the reference or the PEP of the version that added it.
- Raising the floor is the user's choice and is visible in `typy.toml`, so a
  reader of the project sees which CPython versions it supports.

## Related requirements

STD-01 to STD-05, CLI-12, MOD-03, MOD-06 to MOD-09, NFR-10, A-02, OI-11,
ADR-0002, ADR-0005.
