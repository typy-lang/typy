# ADR-0017: Warn where CPython warns while compiling

## Status

Accepted

## Date

2026-11-12

## Context

TyPy 0.3 adds warnings and `--deny-warnings` (DIA-07). A warning reports a
program that TyPy accepts and compiles with CPython's meaning, but that is
probably not what its author meant. Which constructs get one decides what
TyPy's warnings are for.

CPython 3.12 to 3.15 give a `SyntaxWarning` while compiling for these
constructs:

| Construct | CPython | TyPy before 0.3 |
| --- | --- | --- |
| An invalid escape sequence, such as `"\d"` or `b"\q"` | Warns; the backslash is kept | Accepted silently |
| An octal escape above `\377`, such as `"\777"` | Warns | Accepted silently |
| `assert (test, "message")`, a tuple with items | Warns; always true | Accepted silently |
| `x is 1`, `x is "a"`, `x is ()` | Warns | E0301 |
| `return`, `break` or `continue` in `finally`, from 3.14 | Warns | E0419 |

Running `python3.X -W always` over a file of each construct showed the
same set in every version from 3.12 to 3.15; 3.14 rewords the escape
warnings.

## Options considered

1. **Warn where CPython warns while compiling, and nowhere else.** Each
   warning is one a Python developer already knows, and TyPy adds no
   opinion of its own about style.
2. **Add TyPy's own lint warnings**, such as an unused variable or
   unreachable code. Useful, but Ruff and other linters already give them
   for the same source, and each one would be a judgement TyPy makes
   beyond CPython (ADR-0002, ADR-0003).
3. **No warnings in 0.3.** DIA-07 would have nothing to test, and the
   escape mistakes would stay silent.

## Decision

TyPy will use option 1. A program TyPy accepts gets a warning wherever
CPython 3.12 to 3.15 give a `SyntaxWarning` while compiling it (DIA-08).
Constructs TyPy refuses stay errors.

- **W0101** is an invalid escape sequence, in a string, bytes or f-string
  literal that is not raw, found in the parser's tokens. Writing the
  backslash twice keeps the value, so the diagnostic shows that line as
  its fix (DIA-02). An octal escape above `\377` has no such fix, since
  doubling the backslash would change the value.
- **W0301** is `assert` of a tuple display with items.
- The messages are TyPy's own sentences (DIA-01), not CPython's wording,
  which changes between versions.
- A warning never fails a build. `--deny-warnings` makes any warning fail
  `check`, `build` and `run` with exit status 1, before code is generated,
  so no executable is written.

A later CPython warning about a construct TyPy accepts gets a code in the
same way, with its range taken from the area of the construct (`W01xx`
lexical, `W03xx` statements).

## Consequences

- TyPy's warnings agree with what `python -W error` would refuse at
  compile time, so a program that is clean under one is clean under the
  other.
- Each warning comes with a known cause and, where it is certain, a fix.
- Style checks stay with linters; TyPy does not compete with them.
- The conformance oracle runs CPython with `-W ignore::SyntaxWarning`:
  CPython prints these warnings on standard error while it compiles, and
  TyPy prints them while it compiles, not when the program runs, so the
  program's own output is what the two are compared on.
- Warnings travel with a successful result through `Project::warnings`,
  `TypedProgram::warnings` and `Success::warnings`, and `typy run` checks
  a build it takes from its cache again, so the warnings show on every
  run.

## Related requirements

DIA-01, DIA-02, DIA-07, DIA-08, LEX-05, LEX-06, LEX-07, SMT-04, ADR-0002,
ADR-0003.
