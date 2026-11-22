# ADR-0094: Bind the operations of an `Any` value when the program runs

## Status

Accepted

## Date

2026-11-21

## Context

Typed Python uses `typing.Any` where a type cannot be written: typeshed's
standard library stubs name it in 5.6% of parameters and 4.8% of results,
`json.loads` among them, and a program may read `data["key"]` from such a
result. TYP-10 rejects `Any` in 0.10 (OI-35).

C# 4.0's `dynamic` binds each operation when it runs, through a cache at
the call site, so only code that uses it pays; the rest of the program
stays statically compiled. ADR-0090 holds a value of type `object` as an
object that knows its type.

## Options considered

1. **Keep rejecting `Any`.** `object` with narrowing is the only open
   type.
2. **Treat `Any` as `object`.** Every use would need narrowing, which
   typed Python code does not write.
3. **Bind the operations of an `Any` value when the program runs.**

## Decision

Option 3, for 0.13.

- An `Any` value is held as an `object` (ADR-0090).
- An attribute read, an assignment to an attribute, a method call, an
  operator, a subscript, a call, iteration and `len` on an `Any` value are
  bound when they run, by the value's type ID or class: an attribute by
  its name in a table the class emits, an operator by the types of its
  operands, a call by the function's parameters. Each operation caches
  the binding for the type it last saw.
- An operation the value does not support raises the `TypeError` or
  `AttributeError` CPython raises, with its text.
- A value passes from `Any` to a declared type through the check of
  ADR-0091, and any value passes to `Any`.
- A class emits its name table only when the program has an `Any` value
  that may hold its instances.

## Consequences

- Code that uses `Any` is slower than typed code, by a lookup and a check
  for each operation; code that does not use it is unchanged.
- Safety holds: every bound operation checks the types it reads.
- The binary grows by the name tables of the classes an `Any` value may
  reach.
- The checker no longer proves operations on `Any` values; their errors
  come when the program runs, as in CPython.

## Related requirements

TYP-25, TYP-10, TYP-23, TYP-24, NFR-07, OI-35, ADR-0090, ADR-0091.
