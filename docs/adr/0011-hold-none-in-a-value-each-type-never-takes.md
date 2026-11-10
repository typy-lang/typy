# ADR-0011: Hold None in a value each type never takes

## Status

Accepted

## Date

2026-11-09

## Context

Release 0.2 accepts the annotation `T | None` (TYP-06) and narrows such a
value to `T` after `is None` and `is not None` tests (TYP-13). A value of
type `T | None` is stored in variables, passed to functions, returned, and
held as an item of a list, a tuple, a set or a dict, so every layer needs a
way to tell `None` from a value of `T`.

Until now `None` has had no run-time value: an expression of type `None`
lowers to `unit`. Each other type has a fixed layout: an `int` is a tagged
eight-byte word, inline when its lowest bit is 1 and otherwise the address
of a large integer; a `float` is an IEEE 754 double; a `bool` is one bit in
registers and one byte in a tuple; a `str`, `bytes`, list, set or dict is
the address of its object; a tuple is held as its items (ADR-0001,
ADR-0009). Lists and dicts store each item in eight bytes, or a tuple
inline, and the runtime reads an item by its `ItemKind`.

## Options considered

1. **A heap box.** A `T | None` is the address of a box holding the `T`, or
   null for `None`. One rule serves every `T`, but each `int | None` or
   `float | None` that holds a value allocates, and every read follows a
   pointer.
2. **A flag beside the value.** A `T | None` is a pair of a presence flag and
   a `T`. The rule is uniform, but an `int | None` takes sixteen bytes, and
   lists and dicts need a second layout for optional items.
3. **A value the type never takes.** Each `T` keeps its own layout and holds
   `None` in a bit pattern it never uses: the word 0 for an `int`, since an
   inline `int` has its lowest bit set and a large one is a non-null
   address; the null address for an object; for a `float`, the signalling
   NaN `0x7ff4000000000001`, which no float operation produces, because
   arithmetic and the C library give quiet NaNs and a program has no way to
   build a double from bits; the byte 2 for a `bool`. A tuple has no spare
   value, so a `tuple | None` is a pair of a presence flag and the tuple.
   `None | None` is `None`.

## Decision

TyPy uses option 3. TyIR has a type for each optional layout, so a value
keeps its optional type inside tuples and lists, and the runtime has an
item kind for each, which writes, compares and hashes `None` as CPython
does. Narrowing a value to `T` is free for every layout but the tuple's,
and giving a `T` where `T | None` is expected is free for all but `bool`,
which widens from one bit to a byte, and the tuple, which gains its flag.

## Consequences

- An `int | None`, a `float | None` and an object `| None` take eight bytes,
  in a variable and in a container, and `is None` is one comparison.
- The runtime's reference counting accepts the word 0 and the null address
  and does nothing for them, so an optional value is counted like its `T`.
- A float operation must never produce the signalling NaN TyPy reserves;
  `float.fromhex`, `struct` and other ways to build a double from bits are
  not part of TyPy, and adding one must keep the pattern out.
- An optional tuple is laid out as a two-item tuple of a `bool` and the
  tuple, so it reuses the tuple descriptors of ADR-0009.

## Related requirements

TYP-06, TYP-13, EXP-10, DAT-04, ADR-0001, ADR-0009.
