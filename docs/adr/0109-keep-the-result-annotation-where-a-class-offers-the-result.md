# ADR-0109: Keep the result annotation where a class offers the result

## Status

Accepted

## Date

2026-11-23

## Context

ADR-0092 infers a function's result from its `return` statements. A
generator, a function that calls itself, a method that overrides or is
overridden, and a function a library exports keep their result
annotations. The checker also reads some results before it checks any
body, so a result inferred from the body would come too late:

- A static or a class method gets a bridge function when its class's
  members are collected. The bridge has the method's signature, so a
  class object and a subclass call it alike.
- A special method such as `__len__` or `__eq__` gives its class an
  operator or a protocol, which the checker reads while it collects the
  class.
- A property's result is the type of an attribute of its class.
- A method of a generic class is copied for each set of type arguments,
  and a method of a protocol stands for the methods of the classes that
  satisfy it, before any body is checked.
- A nested function is checked after the function around it, and that
  function's body already reads the nested function's result.

## Options considered

1. **Infer each of these too,** by looking at their bodies while the
   class members are collected. The module's functions and variables have
   no types yet at that point, and many such bodies read them, so the
   look fails where the program is correct.
2. **Keep the annotation for each of them,** with E0204, and infer the
   results of module functions and instance methods, which are most of
   what programs leave unannotated.
3. **Infer a nested function's result from a look at its body** when the
   function around it reaches its `def`, inside that function's check.

## Decision

Option 2, with option 3 planned for 0.13 as TYP-33. A static method, a
class method, a special method, a property, a method of a generic class
or of a protocol, and a nested function keep their result annotations.
E0204 says which functions TyPy infers.

## Consequences

- Module functions and instance methods, the common case, read as CPython
  programmers write them.
- The functions that keep the annotation are those whose result is part
  of what a class or a protocol offers, and nested functions until 0.13.
- TYP-26 lists these exceptions, and TYP-33 records the nested functions.

## Related requirements

TYP-01, TYP-26, TYP-33, ADR-0092.
