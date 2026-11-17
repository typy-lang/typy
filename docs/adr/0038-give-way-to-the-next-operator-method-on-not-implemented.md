# ADR-0038: Give way to the next operator method on NotImplemented

## Status

Accepted

## Date

2026-11-16

## Context

ADR-0026 calls an operator or comparison method of the left operand's
class, or the reflected method of the right operand's, and left a method
that returns `NotImplemented` to 0.5. CPython uses the value to pass an
operation on:

- **Arithmetic.** `a + b` calls `a.__add__(b)`; when that returns
  `NotImplemented` and the operands are of different classes, it calls
  `b.__radd__(a)`, and when that returns it too, raises `TypeError:
  unsupported operand type(s) for +: 'A' and 'B'`. A right operand whose
  class derives from the left's and defines `__radd__` again goes first.
  `a += b` calls `a.__iadd__(b)` first and `a + b`'s methods after it.
- **Comparisons.** `a < b` calls `a.__lt__(b)`, then `b.__gt__(a)`, for
  operands of one class too. The reflected method goes first whenever the
  right operand's class derives from the left's, whether it defines the
  method again or not. When every method gives way, `==` compares
  identities, `!=` gives `is not`, and an ordering raises `TypeError: '<'
  not supported between instances of 'A' and 'B'`.

A method returns `NotImplemented` where its annotation says `bool` or the
class: type checkers accept it because typeshed makes `NotImplementedType`
a subclass of `Any`. TyPy has no `Any` (TYP-10), so the value needs a type
of its own. A call through the method table expects one return type in
every class of a chain of bases (ADR-0021). A list, a set or a dict
compares instances through the `equal` slot of their type object
(ADR-0026), which returns a byte and cannot run a chain of calls.

## Options considered

1. **A return type that takes `NotImplemented`, and steps that test it.**
   A scan of the whole program finds the operator and comparison methods
   whose body says `return NotImplemented`. Every method of that name in
   the same chain of bases returns `T | NotImplementedType`. An operator
   call tests each result and passes on to the next method, then to the
   fallback. The `equal` slot calls a method the checker writes for
   `self == other`.
2. **Raise an internal exception for `return NotImplemented`.** The
   operator would catch it and call the next method. A direct call
   `a.__eq__(b)` would then raise instead of returning the value, and every
   operator call would pay for a handler.
3. **Keep refusing** `NotImplemented` until 1.0.

## Decision

TyPy uses option 1.

- **The value.** `NotImplemented` is one immortal object of the runtime
  type `NotImplementedType`, written as CPython writes it.
- **Signatures.** `check::not_implemented` reads every module before any is
  checked. A method of an arithmetic, reflected, in-place or comparison
  name whose body holds `return NotImplemented` marks its name in its chain
  of bases, and each method of that name in the chain returns
  `T | NotImplementedType`. A method that returns both `None` and
  `NotImplemented` is planned for 1.0 (CLS-33). `return NotImplemented`
  elsewhere is E0405.
- **Operator calls.** `OperatorCall` names its kind: arithmetic, ordering or
  equality. Lowering builds the steps CPython takes: the in-place method,
  the choice of which method goes first, a test that the operands are of
  different classes before an arithmetic reflected method, and the
  methods. A result that may be `NotImplemented` is tested and, when it is,
  the next step runs. After the last step, equality compares identities
  and any other operator raises CPython's `TypeError`.
- **Containers.** A class whose `__eq__` may give way gets `<equal>`, a
  method that returns `self == other` through these steps, and its `equal`
  slot calls `<equal>`. An ordering method that may give way would make a
  sort raise, which a slot cannot pass on, so it is E0201 until slots may
  raise (CLS-22).

## Consequences

- A method that may give way returns a tagged union where it returned a
  scalar, so a call of it costs the union and a test of its tag. A method
  that never returns `NotImplemented` costs nothing new.
- A comparison of instances whose classes may derive from one another tests
  the right operand's class first, as CPython does. An arithmetic reflected
  method on two instances runs only after a test of their classes.
- A direct call such as `a.__eq__(b)` has the type
  `bool | NotImplementedType`, and `result is NotImplemented` tests it.

## Related requirements

CLS-07, CLS-20, CLS-21, CLS-22, CLS-33, TYP-10, ADR-0021, ADR-0026.
