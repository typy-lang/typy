# ADR-0039: Keep a slot method's exception until the runtime call returns

## Status

Accepted

## Date

2026-11-16

## Context

ADR-0026 has lists, sets, dicts, sorts, `min`, `max`, `any` and `all`
call a class's `__eq__`, ordering methods, `__hash__` and `__bool__`
through the `equal`, `order`, `hash` and `truth` slots of its type
object. A slot returns a `bool` or a hash, so a method that may raise was
refused there, and so was a method that prints, since `print` may raise.
CLS-22 accepts them, and an ordering method that returns
`NotImplemented` needs it too, since a sort of operands that every method
gives way for raises `TypeError` (ADR-0038).

In CPython the exception leaves the container operation at once: no
further method runs, a set or a dict is not changed, and the operation's
own error, such as `ValueError: list.remove(x): x not in list`, is never
raised. The runtime calls slots from its list, tuple, set and dict
functions, many of them in loops.

## Options considered

1. **Slots that return an exception.** Every slot, and every runtime
   function that calls one, returns `Raising`, and each loop stops at the
   first exception. It changes the signature of every slot and of most
   container functions, and makes each of their calls an `invoke`, in
   every program.
2. **A pending exception kept by the runtime.** The slot function the
   compiler writes keeps a method's exception with the runtime and returns
   a default. While one is kept, every slot of a class of the program
   returns its default without running its method, the runtime's own
   errors give way to it, and a set or a dict adds nothing. After a runtime
   call that may run a slot, the program raises it.
3. **Keep refusing** such methods until 1.0.

## Decision

TyPy uses option 2.

- **The slot.** For a method that may raise, the slot function tests the
  exception the method returns, gives it to `typy_rt_slot_failed` and
  returns false or 0. In a program with such a method, every slot first
  asks `typy_rt_slot_pending` and returns its default when an exception is
  kept, so no method of the program runs after the first that raised.
- **The runtime.** `slot::or_pending` gives way to the kept exception
  where a function would raise its own error after calling slots:
  `list.remove`, `list.index`, a dict's missing key and `set.remove`. A
  set or a dict that would add or set an item while an exception is kept
  leaves itself unchanged.
- **The program.** When a slot's method may raise, lowering follows each
  runtime call that takes an item, a tuple, a container, an iterator or an
  instance with `slot_raised`, which raises the kept exception. A program
  whose slot methods cannot raise gets no such test.
- **Comparison bridges.** An ordering method that may give way has a
  bridge, `<less>`, `<less_equal>`, `<greater>` or `<greater_equal>`, that
  returns `self op other` through ADR-0038's steps, and the `order` slot
  calls it, as the `equal` slot calls `<equal>`.

## Consequences

- A sort, a search or a set operation over instances whose methods raise
  behaves as in CPython: the first exception propagates, no later method
  runs, and the container keeps its items.
- In a program with such a method, a runtime call on a container costs a
  call that tests the kept exception; other programs cost nothing more.
- The order in which a sort calls `__lt__` becomes visible to a program
  whose method prints or raises. CPython 3.12, 3.13 and 3.15 each compare
  in another order, and TyPy's merge sort in its own (OI-27).

## Related requirements

CLS-07, CLS-21, CLS-22, ADR-0026, ADR-0038.
