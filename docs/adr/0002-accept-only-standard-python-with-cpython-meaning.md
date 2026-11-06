# ADR-0002: Accept only standard Python, with the meaning CPython gives it

## Status

Superseded by ADR-0008

## Date

2026-11-05

## Context

The TyPy-Lang Specification Pack described a language with Python-like
syntax. It added `let`, `var`, `struct`, `T?` and a `?` operator, and it
changed the meaning of valid Python: lists copied on write, `xs[-1]` raised
an error, `len(s)` counted bytes, and `and` returned a `bool`. It treated
Python compatibility as a tiebreaker (its INV-007).

The project owner rejected that approach. TyPy code shall be standard
Python, unmodified. Two questions follow: how far "standard" goes, and what
TyPy does with a valid program it cannot compile faithfully.

The research found that no maintained compiler produces standalone native
binaries from valid Python while keeping CPython's behaviour (research
section 4.6). Codon is the closest in practice, and it changes integer and
numeric semantics on purpose.

## Options considered

1. **A Python-like dialect**, as in the pack. The compiler is free to pick
   convenient semantics. But Python code does not carry over, Python tools
   do not work on it, and a reader who knows Python is misled where the
   meaning differs.
2. **Python syntax with TyPy's own meaning.** Every file parses with
   CPython, so editors and linters work. But a program that looks right can
   behave differently from CPython, and no test can say which behaviour is
   correct.
3. **Python syntax and CPython's meaning, as a strict subset.** TyPy may
   refuse a program, but a program it accepts behaves as it does under
   CPython. CPython then serves as the test oracle for every run test.

## Decision

TyPy uses option 3.

- TyPy accepts only source that CPython 3.12 to 3.15 parse (STD-01).
- An accepted program runs alike under TyPy and CPython 3.12 to 3.15: the
  same standard output, exit status and last line of standard error, on
  every input where CPython finishes without an uncaught exception (STD-02,
  STD-05).
- Behaviour that Python leaves to the implementation is not covered: `id()`
  and `hash()` values, set iteration order, addresses in default `repr()`
  output, and the recursion limit (STD-03).
- A feature that TyPy cannot compile with CPython's meaning is rejected with
  a diagnostic that names the rule. It is never compiled with a different
  meaning (STD-04, DIA-03).
- TyPy 1.0 covers the typeable part of the Python 3.10 language, with PEP
  695 type parameter syntax for generics. Rules cite the Language Reference
  for Python 3.12, whose text matches the oracle's behaviour.

## Consequences

- Every run test has an expected result that nobody writes by hand: run the
  program under CPython. A disagreement is a TyPy defect by definition.
- Python editors, formatters, linters and type checkers work on TyPy code
  unchanged. TyPy ships no formatter of its own.
- A TyPy program can be developed and debugged under CPython, then compiled.
- Restrictions take the place of new semantics. Some correct Python is
  refused: unannotated functions, variables that change type, `eval`,
  metaclasses, `Any`, and `int ** int` with an exponent that is not a
  non-negative literal (EXP-04). Each refusal has a diagnostic and a rule ID.
- Python behaviour that is costly to compile has to be compiled anyway:
  unlimited-size integers (DAT-01), shared mutable lists (DAT-07), code
  point indexing of strings (DAT-05), and `ZeroDivisionError` for floats
  (EXP-03).
- Each later Python version adds work in two places: the oracle matrix, and
  any change of meaning in the new version that TyPy has to follow.

## Related requirements

STD-01 to STD-06, DIA-03, TYP-10, EXP-04, DAT-01, DAT-05, DAT-07, EXP-03,
NFR-10, OI-01, OI-07.
