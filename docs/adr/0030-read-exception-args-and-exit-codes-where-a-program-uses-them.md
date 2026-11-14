# ADR-0030: Read exception args and exit codes where a program uses them

## Status

Accepted

## Date

2026-11-14

## Context

EXC-11 asks for `args` of an exception, the tuple of its arguments, and
EXC-13 for `code` of a `SystemExit`: `None` without arguments, the one
argument, or the tuple of several. A raise anywhere in the program may
give any arguments, so CPython's types are `tuple[Any, ...]` and `Any`.
TyPy has no type for any value: `object` is a type object only.

The runtime keeps an exception's arguments as a tuple with the
descriptor of its layout (ADR-0013), so it can write, compare and test
them without knowing their types when the program is compiled.

Programs mostly print these values, format them, compare them with a
value, as in `stop.code == 2`, and test them, as in `if error.args:`.

## Options considered

1. **Read them where they are written, compared or tested.** Each such
   use is one operation the runtime performs on the exception. Other
   uses wait for a type of any value.
2. **Add a type of any value now**, with values boxed and their type
   read when the program runs, so `args` is `tuple[object, ...]`.
3. **Move both requirements to 0.5** and design that type there.

## Decision

TyPy uses option 1.

- `print`, `str`, `repr` and `bool` of `args` or `code`, `len(args)`, an
  f-string field, a condition, `not`, and `==` and `!=` with any value
  are recognised when the program is checked, for `name.args` of an
  exception and `name.code` of a `SystemExit`, and each becomes
  `TypedExprKind::ExceptionPart` with its operation.
- The runtime reads the part from the exception: `args` of an exception
  it raised is the tuple of its text. A value compared with a part comes
  as a tuple of one, so its descriptor gives its type, and an optional
  or a union is compared as the value it holds.
- Any other use, such as `error.args[0]`, `x = error.args` or
  `len(stop.code)`, is E0201 until EXC-15.

## Consequences

- The common uses work without a new type or boxing, at the cost of a
  runtime call each.
- A program that stores or indexes `args` must wait for 0.5, where a
  type of any value is designed for EXC-15.

## Related requirements

EXC-11, EXC-13, EXC-15, ADR-0013.
