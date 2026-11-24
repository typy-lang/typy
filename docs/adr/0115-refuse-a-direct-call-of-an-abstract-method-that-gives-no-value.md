# ADR-0115: Refuse a direct call of an abstract method that gives no value

## Status

Accepted

## Date

2026-11-23

## Context

CLS-45 accepts classes derived from `abc.ABC` and their `@abstractmethod`
methods (ADR-0093). Most abstract methods have a body of `...`, `pass`
or a docstring alone, such as `def area(self) -> float: ...`. CPython
runs such a body when a call reaches it, and the call returns `None`
where a `float` is declared: `super().area() + 1.0` raises `TypeError`
for `NoneType` and `float`. TyPy holds a `float` as a machine number,
which has no `None`, and refuses any other function whose end a value
of its declared result does not reach (E0405).

A call reaches an abstract method's own body in three ways: through
`super()`, through the class, as `Shape.area(square)`, and through the
function the class holds, read as `Shape.area`. A call through an
instance never does: an instance is made only of a class that overrides
every abstract method, and the call runs the override. mypy reports the
`super()` form as unsafe (`safe-super`).

## Options considered

1. **Refuse an empty body,** as for any other function with E0405. The
   common form of abstract method is refused, and each one needs
   `raise NotImplementedError`.
2. **Accept the empty body and raise at its end,** in place of the
   `None` CPython returns. A program that reaches it prints another
   error than CPython's.
3. **Accept the empty body, and refuse each call that would run it
   directly where its result does not hold `None`,** as mypy does for
   `super()`.

## Decision

Option 3, with the class's place in the program deciding what counts as
abstract, as `ABCMeta` decides it:

- A method's body gives no value when it holds only a docstring, `pass`
  or `...`. Such an abstract method of a class derived from `ABC`, which
  takes the instance, may reach its end. Where its result holds `None`,
  it returns `None` there, as CPython's does, and every call may run it.
- Where its result does not hold `None`, a call through `super()` or the
  class, and the method read through the class as a value, are E0434.
  Its end raises `NotImplementedError`, which no checked program
  reaches, so a mistake in the checker raises rather than run past the
  end of the function.
- An empty static or class method keeps E0405, since a call through the
  class runs it. So does an empty method of an exception class, which
  CPython makes instances of without the test of abstract methods, and of
  a class not derived from `ABC`, where `@abstractmethod` has no effect.
- `@abstractmethod` above `@property`, `@staticmethod`, `@classmethod`
  or a setter is E0434, as CPython raises `AttributeError` when the class
  is defined.

## Consequences

- The usual abstract class compiles as written, and its subclasses'
  overrides run through the class's method table as any other override.
- A program that calls an empty abstract method through `super()`,
  which CPython runs to a `None` of the wrong type, is refused before it
  runs.
- The overrides of an abstract method whose end raises are called as
  methods that may raise (ADR-0021), which costs one test of the result
  of each call.

## Related requirements

CLS-45, CLS-06, CLS-14, TYP-01, ADR-0093, ADR-0021.
