# ADR-0031: Type a file by its literal mode, and a mode value as text

## Status

Accepted

## Date

2026-11-14

## Context

CPython's `open` gives an object whose class depends on its arguments:
`TextIOWrapper` for a text mode, `BufferedReader`, `BufferedWriter` or
`BufferedRandom` for a binary mode, and `FileIO` for a binary mode with
`buffering=0`. typeshed types `open` by overloads on literal modes and a
literal `buffering`, and gives `IO[Any]` or `BinaryIO` when either is a
value only known when the program runs.

TyPy checks every type when it compiles, and has no `Any` (TYP-10) or
`typing.IO` yet. LIB-14 asks for binary modes and for a mode that is not
a literal.

## Options considered

1. **One type per class, from the literal mode and `buffering`.** A mode
   or a binary `buffering` that is not a literal has no type and is
   refused.
2. **A union of the five classes for any `open`**, which a program
   narrows with `isinstance` before it reads or writes.
3. **Option 1, and a mode that is not a literal types as text.** Such a
   mode is checked when the program runs, and one with `b` is refused
   then, since a text file cannot hold bytes.

## Decision

TyPy uses option 3.

- `Type::BinaryFile` has a `BinaryKind` for each binary class, shown by
  CPython's class name. A literal binary mode gives `BufferedReader`,
  `BufferedWriter` or `BufferedRandom`; with the literal `buffering=0` it
  gives `FileIO`.
- A binary mode with a `buffering` that is not a literal is E0201,
  planned for 1.0 with `typing.BinaryIO` (LIB-19). `buffering=1` makes
  CPython warn with the program's file and line, so it is E0404.
- A mode that is not a literal gives a `TextIOWrapper`. The runtime
  checks it with CPython's `ValueError` messages, and a mode with `b`
  raises `TypeError`, which CPython would not raise.
- All five classes are one runtime object, the text file of ADR-0019,
  with flags for binary and for `FileIO`.

## Consequences

- Programs that open text files with a mode held in a variable, a
  common pattern, are accepted.
- A program that opens a binary file through a mode held in a variable
  fails when it runs, where CPython gives a binary file. The guide says
  so.
- Assigning files of two classes to one variable is refused, as mypy
  refuses it.

## Related requirements

LIB-04, LIB-14, LIB-19, TYP-10, ADR-0019.
