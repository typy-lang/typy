# ADR-0026: Call operator methods directly and from the type object slots

## Status

Accepted

## Date

2026-11-13

## Context

CLS-07 asks for operator methods: `a == b` calls `a.__eq__(b)`, `a < b`
calls `a.__lt__(b)`, `a + b` calls `a.__add__(b)`, `len(a)` calls
`a.__len__()`, and so on. CPython also calls them where the program
does not name an operator: `x in items` and `items.index(x)` call
`__eq__`, `sorted(items)` calls `__lt__`, and a set or a dict calls
`__hash__` and `__eq__` on its keys.

The runtime compares, orders and hashes the items of a container
through the slots of each item's type object (ADR-0009, ADR-0012):
`equal`, `order` and `hash`. They return a `bool` or a hash and cannot
pass on an exception.

CPython tries the reflected method when the left operand has none or
returns `NotImplemented`: `a > b` may call `b.__lt__(a)`. A class that
defines `__eq__` and not `__hash__` has no hash, so its instances
cannot be set members or dict keys.

## Options considered

1. **Calls in the program only.** Operators the program writes call the
   methods; containers keep comparing instances by identity. `x in
   items` would then differ from CPython.
2. **Calls in the program, and slot functions in each class's type
   object.** Each class with operator methods gets slot functions that
   call them, so a container compares its items as CPython does.

## Decision

TyPy uses option 2.

- **Operators.** `a op b` on an instance calls the method of `a`'s class
  when it takes `b`, through the method table when a subclass defines it
  again (ADR-0021); otherwise the reflected method of `b`'s class when it
  takes `a`; otherwise it is an error, as CPython raises `TypeError`.
  `a != b` without `__ne__` is `not a.__eq__(b)`. A method returning
  `NotImplemented` is planned for 0.5.
- **Parameters.** A comparison method takes `other` of a type that
  accepts the class that defines it, so every instance of the class and
  of its subclasses is one it takes; an override may not narrow it
  (ADR-0021).
- **What a class offers.** The checker reads, from the class bodies,
  which operator methods each class defines or inherits, and keeps it
  with the class's type: an instance can be ordered when its class has
  `__lt__` or `__gt__`, and hashed unless its class defines `__eq__`
  without `__hash__`.
- **Slots.** A class with `__eq__`, an ordering method or `__hash__` has
  slot functions that call them. The `equal` slot of an instance and an
  object its `__eq__` does not take gives false, as `NotImplemented` and
  CPython's identity fallback give.
- **Methods that raise.** A slot cannot pass on an exception, so a
  comparison or hash method that may raise cannot be called from a
  container; such a class is refused there, planned for 0.5. An operator
  the program writes passes the exception on.

## Consequences

- `x in items`, `sorted(items)`, sets and dict keys of instances behave
  as in CPython.
- A class without operator methods compiles and runs as before.
- An instance's comparison inside a container costs an indirect call
  through its type object, as a `str` item's does.

## Related requirements

CLS-07, EXP-06, DAT-09, DAT-10, ADR-0009, ADR-0012, ADR-0021.
