# ADR-0095: Follow each CPython version from its final release to its end of life

## Status

Accepted

## Date

2026-11-21

## Context

TyPy accepts only source that every oracle version from the program's
Python floor up runs alike (STD-01, ADR-0008). The oracle is CPython 3.12
to 3.15 (OI-01), and `python-floor` takes one of those four (CLI-12).
CPython releases a new version each October: 3.16 is due in October 2027.
CPython 3.12 reaches its end of life in October 2028. Each version adds
language changes, such as PEP 750's template strings in 3.14 and PEP
798's unpacking in comprehensions in 3.15 (OI-37).

## Options considered

1. **Keep CPython 3.12 to 3.15 until TyPy 1.0.** New syntax waits, and
   programs written for a newer Python are rejected.
2. **Add each version from its first beta.** New PEPs land earlier, but
   TyPy follows behaviour that can still change before the final release.
3. **Add each version after its final release, and drop the oldest at its
   end of life.**

## Decision

Option 3.

- The first minor release of TyPy after a CPython version's final
  release adds that version to the oracle and to the values
  `python-floor` takes, and the conformance suite runs on it.
- Each language change of that version becomes a requirement that names
  the lowest Python floor it needs, planned for a release as any other.
- The lowest floor stays 3.12 until CPython 3.12's end of life. The first
  minor release after a version's end of life stops accepting it as a
  floor; its changelog and migration notes say which floor to set
  instead.
- Messages, `sys.version` and other wording follow the newest oracle, as
  OI-14 decides.

## Consequences

- A program keeps compiling for about five years from the CPython
  version it targets, the time CPython supports that version.
- Each new oracle adds a conformance run to CI.
- A requirement whose behaviour differs between oracle versions keeps
  following the program's floor (ADR-0008).

## Related requirements

STD-08, STD-01, CLI-12, OI-01, OI-14, OI-37, ADR-0008.
