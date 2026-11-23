# ADR-0119: Provide the standard library modules that need no operating system first

## Status

Accepted

## Date

2026-11-23

## Context

ADR-0097 starts the `typy-stdlib` crate in 0.12 with nine modules,
`textwrap`, `base64`, `heapq`, `bisect`, `statistics`, `time`, `random`,
`copy` and `string`, each written in typed Python that CPython also runs,
with a test that compares their signatures with typeshed's stubs. Three
of them reach the operating system: `time` reads clocks, `random` seeds
itself from the system's randomness, and `statistics` uses `random`. They
need the private module `_typy`, which ADR-0007 decides and which does
not exist. The other six are computation on values TyPy already has.

`frozendict` (DAT-18) is a built-in from Python 3.15 that a typed Python
class can give, and needs a home that every module of a program shares.

## Options considered

1. **Keep all nine in 0.12,** with `_typy`.
2. **Provide the six modules that need no operating system in 0.12,**
   with the crate, the way a program gets a module of it, and the
   signature test, and the three that need `_typy` in 0.13.
3. **Move the whole group to 0.13.**

## Decision

Option 2. The decisions of ADR-0097 stand for every module: each is typed
Python that CPython also runs, a test compares its signatures with a
pinned typeshed revision and lists each difference kept on purpose with
its reason, and conformance tests compare it with CPython's module.

- 0.12 gives `bisect`, `heapq`, `copy`, `string`, `textwrap` and
  `base64` (LIB-27), and the crate and its loading, which `frozendict`
  also uses (DAT-18).
- 0.13 gives `statistics`, `time` and `random`, with `_typy` (LIB-30).

## Consequences

- 0.12 ships the crate and its test on the modules that exercise the
  checker most, without a module of system calls.
- A program that imports `statistics`, `time` or `random` is refused with
  E0201, naming 0.13 and LIB-30.

## Related requirements

LIB-27, LIB-30, DAT-18, ADR-0097, ADR-0007.
