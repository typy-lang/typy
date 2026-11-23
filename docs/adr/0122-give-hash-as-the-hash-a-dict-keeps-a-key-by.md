# ADR-0122: Give hash as the hash a dict keeps a key by

## Status

Accepted

## Date

2026-11-23

## Context

No requirement gave the builtin `hash`, so a program that called it was
refused with E0202. Two needs make it one:

- The common way to write `__hash__` is `return hash((self.x, self.y))`,
  the hash of a tuple of the attributes `__eq__` compares.
- `frozendict` (DAT-18), written in typed Python, hashes its items to
  hash itself.

The runtime already hashes every key of a set or a dict as CPython does
for an `int`, a `float`, a `bool`, `None` and tuples of them, and with
CPython's function under a fixed key for `str` and `bytes` (ADR-0010).
The `__hash__` a dataclass gets uses that hash through the checker's
`Hash` expression (CLS-16). STD-03 leaves hash values out of what TyPy
promises.

The hash slot of a class reduced what `__hash__` returned modulo
`2**61 - 1`. CPython keeps that value when it fits a word and hashes the
`int` only when it does not, so `hash(Point(1, 2)) == hash((1, 2))`
holds in CPython and did not in TyPy.

## Options considered

1. **Keep `hash` excluded,** and write `frozendict` with a hidden hash.
   The idiom of `__hash__` stays refused, and the stdlib module would no
   longer be Python that CPython runs.
2. **Give `hash` as the hash the runtime keeps a key by,** refuse a value
   of a type without a hash as a set item is refused, and make the class
   slot keep what `__hash__` returns as CPython's does.

## Decision

Option 2.

- `hash(value)` is the `Hash` expression of the checker, which the
  runtime computes. It takes one argument, by position.
- A value of a type that has no hash, such as a list or a tuple that
  holds one, is E0413. In a copy of a generic function or class, whose
  type arguments a call chose, it raises CPython's `TypeError:
  unhashable type: 'list'` where it runs instead, since CPython raises
  only when the value is hashed. A type that may hold a hashable value,
  such as `list[int] | None`, raises for every value there.
- The class slot keeps what `__hash__` returns when it fits 64 bits,
  turns -1 into -2, and hashes a larger `int`; `hash()` returns a hash
  outside the inline range as a large `int`.

## Consequences

- `__hash__` written as CPython code is written compiles and gives
  CPython's hash for tuples of numbers.
- A set of instances whose `__hash__` returns values of 61 bits or more
  may iterate in another order than before; STD-03 does not promise it.
- `frozendict` can be written in typed Python that CPython also runs.

## Related requirements

LIB-34, DAT-18, CLS-07, CLS-16, STD-03, ADR-0010.
