# ADR-0097: Check the standard library's signatures against typeshed

## Status

Accepted

## Date

2026-11-21

## Context

ADR-0007 decides that each standard library module TyPy provides is typed
Python that TyPy compiles and CPython runs, with the operating system
reached through a private module `_typy`. No such module exists yet:
`math`, `sys`, `typing`, `dataclasses` and `ctypes` are tables in the
checker (ADR-0018). OI-41 asks when the `typy-stdlib` crate starts, with
which modules, and how its signatures stay those typed Python expects.

Typed Python code is checked against typeshed, whose stubs give the types
of every standard library function. The research note "newer Python and a
typed standard library" counted their typing forms: a first group of
modules, `textwrap`, `base64`, `heapq`, `bisect`, `statistics`, `time`,
`random`, `copy` and `string`, needs nothing TyPy lacks beyond `object`.

## Options considered

1. **Read typeshed's stubs when a program compiles.** The checker would
   need every typing form the stubs use before any module could be
   provided.
2. **Write each module's signatures by hand, unchecked.** They could drift
   from typeshed, so code that mypy accepts could be refused.
3. **Write each module's signatures in the module, and compare them with
   typeshed in a test,** as mypy's `stubtest` compares stubs with modules.

## Decision

Option 3, starting in 0.12 with the first group.

- The `typy-stdlib` crate and `_typy` start in 0.12, after `object`
  (TYP-23) lands in 0.11, with `textwrap`, `base64`, `heapq`, `bisect`,
  `statistics`, `time`, `random`, `copy` and `string` (LIB-27).
- A test reads typeshed's stub of each module TyPy provides, at a pinned
  typeshed revision, and fails when a public name is missing or its
  signature accepts other arguments or gives another result. A difference
  TyPy keeps on purpose is listed in the test with its reason.
- Each module also has conformance tests that compare it with CPython's
  module of the same name, as ADR-0007 sets out.
- Later groups follow the typing forms they need: `@overload`, `Self` and
  `Literal` in 0.12, `Any` in 0.13.

## Consequences

- Each module is ordinary typed Python that CPython can run, so the
  module and its tests show the same results on both.
- Raising the pinned typeshed revision is a change of its own, which may
  ask for changes to the modules.
- `time` and `random` come in 0.12 rather than 1.0; `os.environ` and
  `os.path` stay planned for 1.0 (LIB-07).

## Related requirements

LIB-27, LIB-07, LIB-09, TYP-23, OI-41, ADR-0007, ADR-0018.
