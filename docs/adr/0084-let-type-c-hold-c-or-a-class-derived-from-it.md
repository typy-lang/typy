# ADR-0084: Let `type[C]` hold `C` or a class derived from it

## Status

Accepted

## Date

2026-11-20

## Context

`isinstance(shape, cls)` with a class held in a variable or computed, and
`issubclass`, are planned for 0.10 (CLS-18, CLS-19). Both need to know
what a value of type `type[C]` may hold. In TyPy up to 0.9 it holds
exactly `C`: the detailed design gives `type[X]` "the one of `X`", and
`cls: type[Shape] = Square` is E0402. No decision record says so, and
PEP 484 reads `type[C]` as `C` or any class derived from it, as mypy and
pyright do.

Holding exactly `C` also produces a wrong result. With `c: Shape =
Circle()`, `type(c)` is typed as the exact `type[Shape]`, so `type(c)()`
makes a `Shape` in TyPy, where CPython 3.12 to 3.15 make a `Circle`.

CPython 3.12.16, 3.13.18, 3.14.10 and 3.15.2, checked on 2026-11-20,
agree on these points:

- `isinstance(x, cls)` and `issubclass(sub, cls)` take a class, a tuple
  of classes, nested tuples, or a union such as `int | Square`, and test
  the tuple's items in order, so `(Square, 3)` is true for a `Square` and
  raises `TypeError` for anything else.
- `issubclass(bool, int)` is true and `issubclass(int, float)` false.
- `case cls():` takes any name or dotted name as the class, read when the
  match runs.
- `issubclass(list[int], list)` is false in 3.12 and raises `TypeError:
  issubclass() arg 1 must be a class` from 3.13.
- A class pattern whose name holds no class raises `TypeError: called
  match pattern must be a class` up to 3.14 and `class pattern must refer
  to a class` in 3.15.

## Options considered

1. **Keep `type[C]` exact,** so every test against it is decided when the
   program compiles. A variable could hold only one class, contrary to
   PEP 484, and `type(x)` would have to become the plain `type` to stop
   the wrong construction.
2. **`type[C]` holds `C` or a class derived from it,** with an exact form
   kept inside the compiler for a class's own name and for `cls` in a
   class method's copy (ADR-0025).

## Decision

Option 2.

- **Assignment.** `type[Sub]` passes as `type[C]` when `Sub` derives from
  `C`, unchanged, since a type object is one pointer. `type(x)` of a
  value whose static class is `C` is `type[C]`, not exact.
- **Calls.** Calling a `type[C]` that may hold a subclass, and reaching a
  class method or class variable through it, is E0201, planned for 0.14
  (CLS-39), since a subclass's `__init__` may take other arguments and
  separate compilation does not know every subclass (ADR-0016). This
  refuses `type(c)()`, which 0.9 accepted and ran with the wrong class;
  calling the class by its name keeps working. `__name__`, `__qualname__`,
  `__module__`, `repr`, `==`, `is` and `hash` read the type object the
  value holds.
- **`isinstance(x, cls)`** with a class held in a value calls
  `typy_rt_is_instance` with the type object; a value without a header,
  such as an `int` or a fixed tuple, is tested by its static type object.
  A tuple of classes may be a value of type `tuple[type[C], ...]` or a
  fixed tuple of type objects, tested in order as CPython does; a union
  written in the call, such as `int | Square`, is read as `Union[int,
  Square]` (TYP-17).
- **Narrowing.** Where the test holds, the value narrows to `C`, the bound
  of the class value, intersected with its static type, and the members
  of a union that cannot derive from `C` drop out. Where it fails,
  nothing is removed, since the class may be a subclass. A plain `type`
  narrows nothing.
- **Class patterns.** `case cls(...)` with a class held in a value
  accepts keyword sub-patterns, which are attributes of `C`. Positional
  sub-patterns, which read `__match_args__` of the class the value holds,
  are planned for 0.14 (CLS-40).
- **`issubclass(sub, cls)`** is a constant where both classes are exact or
  the bound decides it, and otherwise calls the runtime's `derives_from`
  on the two type objects. It takes the same second arguments as
  `isinstance`, narrows `type[...]` values as `isinstance` narrows
  instances, and takes the abstract collections of LIB-20 (ADR-0086).
  `issubclass(list[int], list)` is E0202 below floor 3.13, where the
  oracles differ, and E0401 from it.
- **Errors known when compiling,** such as a non-class argument, are
  E0401 with CPython 3.15's text.

## Consequences

- `cls: type[Shape] = Square`, a list of classes, `isinstance` with a
  computed class and `issubclass` work as in CPython and as type checkers
  read them.
- About 18 places in the checker that read a type object's class now
  tell the exact form from the bounded one.
- `type(c)()`, and a call of a `type[C]` parameter such as `cls()` in
  `def make(cls: type[Shape])`, are refused until 0.14 where 0.9 accepted
  them. The changelog names the change, with the replacements: call the
  class by its name, or pass a `Callable[[], Shape]` such as a function
  that makes one.

## Related requirements

CLS-18, CLS-19, CLS-12, CLS-09, CLS-39, CLS-40, TYP-17, LIB-20, ADR-0008,
ADR-0012, ADR-0016, ADR-0025, ADR-0086.
