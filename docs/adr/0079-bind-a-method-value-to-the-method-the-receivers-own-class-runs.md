# ADR-0079: Bind a method value to the method the receiver's own class runs

## Status

Accepted

## Date

2026-11-20

## Context

0.8 made `obj.method` a value: a function object that captures the
instance and calls the method through the method table (ADR-0060). Four
kinds of method value were left for 0.10 (FUN-09), because the method a
value must run is not the one its static type names:

- a class method, `Box.make` or `box.make`, of which each class has its
  own copy (ADR-0025), reached through an instance by a bridge
  (ADR-0037);
- a static method a subclass defines again, read through an instance;
- a method read through a protocol value, which a call reaches through a
  dispatcher over the classes that satisfy the protocol (ADR-0041);
- a method read through an instance whose subclass overrides it with a
  parameter or result held otherwise, which a call reaches through an
  adapter in the method table (CLS-04).

CPython 3.12.16 and 3.15.2, checked on 2026-11-20, give these values:

- `Box.make` is a bound method of the class: `<bound method Box.make of
  <class '__main__.Box'>>`, with `__self__` the class. Read through
  `box: Box = BigBox(3)`, it binds `BigBox`, so it makes a `BigBox`.
- `Box.make == Box.make` is true and `is` is false; `Box.make ==
  BigBox.make` is false, and `box.make == BigBox.make` is true, with equal
  hashes.
- A static method read through a `Box`-typed `BigBox` instance is the
  subclass's plain function, and read through the class it is `Box`'s.
- A protocol value's method is the bound method of the instance's own
  class, `<bound method Square.area of ...>`, equal to `square.area`.

## Options considered

1. **One value for the static type,** binding the bridge, the dispatcher
   or the adapter. Calls run the right code, but `repr`, `==` and `hash`
   name the static class's method, so `shape.area == square.area` is
   false where CPython says true.
2. **Pick the method of the receiver's own class when the value is made,**
   by member tests on the receiver's class, the most derived first, and
   bind that method, through an adapter where its types are held
   otherwise.

## Decision

Option 2.

- **Class methods.** A class method read through a class is a bound
  method object that binds the class's type object and calls that class's
  copy. Read through an instance, the value picks the copy of the
  instance's class by member tests and binds `type(instance)`. `repr`,
  `==` and `hash` go by the copy's identity key and the bound class
  (ADR-0076).
- **Static methods a subclass defines again.** Read through an instance,
  the value picks the function of the instance's class by member tests
  and binds nothing; read through a class, it is that class's function.
- **Protocol methods.** Read through a protocol value, the value picks
  the method of the instance's class among the classes that satisfy the
  protocol, as the dispatcher's arms do, and binds the instance, with an
  adapter when the method's types are held otherwise than the protocol's.
- **Overrides held otherwise.** The value binds the adapter that the
  method table already holds for the override, whose identity key and
  qualified name are the override's.
- **Class methods of a generic class** read through an instance stay
  planned for 0.14 (CLS-30).

## Consequences

- Every method value has CPython's `repr`, equality and hash, and calls
  the method CPython would call.
- Making such a value costs a short chain of member tests, as a call
  through an instance already does; calling it costs what a call through
  any function value costs.
- An adapter made for a protocol method is one more hidden function for
  each class and protocol pair that a program reads as a value.

## Related requirements

FUN-09, FUN-05, CLS-04, CLS-25, CLS-30, ADR-0021, ADR-0025, ADR-0037,
ADR-0041, ADR-0060, ADR-0076.
