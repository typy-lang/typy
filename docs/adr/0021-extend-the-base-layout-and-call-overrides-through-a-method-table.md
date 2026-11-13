# ADR-0021: Extend the base layout and call overrides through a method table

## Status

Accepted

## Date

2026-11-13

## Context

CLS-04 asks for single inheritance between classes of the program, with
each overriding method checked against the base's; CLS-09 asks for
`isinstance` with a class or a tuple of classes, and TYP-07 for
narrowing after it. Until 0.4 a class derives from `object` or from an
exception class (CLS-03, CLS-13), and redefining a base's method other
than `__init__`, `__str__` and `__repr__` is refused.

What the object model gives today (ADR-0012):

- An instance is its header, with the type ID shared by every instance,
  then a pointer to its class's type object, then its attributes, laid
  out as a tuple descriptor of the class describes them.
- An instance is freed through the descriptor of its own class, found
  from that pointer, so it is freed correctly whatever static type the
  variable holding it has.
- An exception class whose base is a class of the program already holds
  the base's attributes first, then its own, so every attribute of the
  base keeps its offset.
- A method call is a direct call to the function the checker resolved
  from the receiver's static class.

Once a variable of type `Shape` can hold a `Circle`, `shape.area()` must
run `Circle.area`, which the static class does not name.

## Options considered

1. **A method table in each type object.** Each class has a table of
   its methods; a subclass starts from a copy of its base's table,
   replaces the entries it overrides and appends its new methods, so a
   method has the same position in every class that has it. A call
   loads the instance's class, then the entry, then calls it.
2. **A switch on the class.** A call compares the instance's class with
   each class of the program that overrides the method. It needs the
   whole program at every call and grows with the hierarchy, and a
   module compiled alone (ADR-0016) cannot see its subclasses.
3. **A lookup by name, as CPython does.** Each call searches the class's
   dictionary and then its bases'. It needs method names at run time and
   costs a hash lookup per call.

## Decision

TyPy uses option 1, and calls directly where no override is possible.

- **Layout.** A subclass's attributes follow its base's, as for
  exception classes, so code that knows only the base reads and writes a
  subclass instance at the same offsets. A subclass may declare an
  attribute its base has only with the same type.
- **Method table.** The type object gains a pointer to the class's
  method table. A call through a value of static class `C` loads the
  instance's class and calls the entry of the method's position in
  `C`'s table.
- **Direct calls.** The program is compiled whole (ADR-0015), so a call
  whose method no class below the static class overrides is a direct
  call, as today. A program without overrides compiles as before.
- **Checked overrides.** An override takes the base method's parameters
  in the same order, with the same names and kinds, and a type each that
  accepts the base parameter's type; it returns a type the base's return
  type accepts. These are the rules of PEP 484 type checkers. Where an
  override's calling convention differs from the base's, as for a
  `bool` result where the base returns `int`, the table holds an adapter
  that converts the arguments and the result.
- **`isinstance`.** For a class it compares the instance's class with
  the given class and then with each base up the chain; for a runtime
  type it compares the type ID, `bool` counting as an `int`. A tuple of
  classes tests each in turn.
- **`super()`.** The zero-argument form, and the two-argument form that
  names the enclosing class and `self`, call the base's method directly,
  since the base is known when the program is compiled.
- **Out of scope.** Multiple inheritance, and deriving from a built-in
  type other than an exception class, such as `list` or `int`, stay
  refused with an error that names CLS-04.

## Consequences

- An instance does not grow; a class gains one table, as large as its
  count of methods that some subclass overrides.
- A call that may reach an override costs two loads and an indirect
  call; any other call stays direct and can be inlined.
- `__repr__`, `__str__` and the operator methods of CLS-07 keep their
  slots in the type object, and an override of one fills the subclass's
  slot, so the runtime calls it.
- Under separate compilation (ADR-0016) a module cannot see its
  subclasses, so development builds call through the table every method
  of a class that is not final; release builds, compiled whole, keep the
  direct calls.
- `isinstance` with a class costs a walk up the chain of bases, as long
  as the hierarchy is deep.

## Related requirements

CLS-04, CLS-06, CLS-07, CLS-09, CLS-13, TYP-07, ADR-0012, ADR-0015,
ADR-0016.
