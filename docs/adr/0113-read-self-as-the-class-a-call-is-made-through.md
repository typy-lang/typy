# ADR-0113: Read `Self` as the class a call is made through

## Status

Accepted

## Date

2026-11-23

## Context

TYP-30 accepts `typing.Self` in a method's parameters and result as the
class the method is called through: `Circle().grown(2.0)` of a method
`Shape.grown(self, factor: float) -> Self` gives a `Circle`. TyPy compiles
one body for each method, which a derived class inherits, and holds an
instance of any class of the program as a pointer to its object, so an
instance of `Circle` and one of `Shape` are held alike.

A class method is copied for each class that has it (ADR-0025), and its
`cls` holds that class exactly.

## Options considered

1. **Copy a method that names `Self` for each class it is called
   through**, as generic functions are copied. Each copy returns its own
   class, and the code grows with each derived class.
2. **Compile the method once for its own class,** check its body with
   `Self` a type of its own that only the instance a call is made through
   has, and read a call's result as the class of the instance it is made
   through.
3. **Read `Self` as the method's own class**, as an annotation of that
   class would. A call through a derived class gives the base, and
   `Circle().grown(2.0).label()` is refused.

## Decision

Option 2 for instance methods, and the class itself for class methods,
whose copies are each their own class's. `Type::SelfType` holds the
method's class and stands in declared types only, as a literal type does
(ADR-0110): an expression's type is the class. In the body, a value
passes where `Self` is declared when it is `self`, a parameter declared
`Self`, or what a method returning `Self` gives through one of them;
another instance of the class is E0433, since a call through a derived
class would get it in place of an instance of that class.

A call through an instance reads the method's signature with `Self` as
the instance's type, so an argument for a `Self` parameter is an instance
of that class, and its result is read as that class through the checked
cast `typing.cast` uses (ADR-0091), whose test of the class costs little.
A call through the class or `super()` reads `Self` as the method's own
class.

## Consequences

- A method that returns `Self` compiles once, and a call through a
  derived class gets the derived class's type with no copy of the body.
- `Self` inside another type, such as `list[Self]`, in a static method, in
  a generic or a protocol class, and in an attribute's annotation are
  planned for 0.13 (TYP-36).

## Related requirements

TYP-30, TYP-36, TYP-24, CLS-06, ADR-0025, ADR-0110.
