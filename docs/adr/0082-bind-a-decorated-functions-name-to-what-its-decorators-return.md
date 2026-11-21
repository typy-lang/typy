# ADR-0082: Bind a decorated function's name to what its decorators return

## Status

Accepted

## Date

2026-11-20

## Context

A decorator is an expression written `@expression` above a `def`; when
the `def` runs, the function is passed to it and the name is bound to
what it returns (Language Reference 8.7, PEP 614). Up to 0.9 TyPy accepts
only `@property`, `@name.setter`, `@staticmethod`, `@classmethod` and
`@dataclass`, and refuses others with E0201 (FUN-06).

CPython 3.12.16 and 3.15.2, checked on 2026-11-20, agree on these points:

- The decorator expressions are evaluated top to bottom, then the
  defaults, and then the decorators are applied bottom up. `@repeat(3)`
  calls `repeat(3)` once, when the `def` runs.
- The name is bound to the outermost decorator's result: `inc.__name__`
  is `'wrapper'` and its qualified name is `trace.<locals>.wrapper`. The
  result may have another signature, or be no function at all, such as an
  `int`. A registering decorator returns the function itself.
- A read of the name inside the function's own body reads that binding,
  so a recursive call goes through the wrapper.
- A decorator that raises leaves the name unbound, and a later read
  raises `NameError`.
- `inc(value=2)` works when the wrapper names its parameter `value`.

On methods the versions differ. `@classmethod` over `@property` gives the
property's value in 3.12 and a bound method in 3.13 and later, and
`@classmethod` over `@staticmethod` returns a value in 3.12 and raises
`TypeError` in 3.15, since 3.13 removed the chaining of descriptors under
`classmethod`. A wrapper over `@staticmethod` turns it into an instance
method.

## Options considered

1. **The name is a variable of the decorator's result type.** Calls by
   name go through the value, with positional arguments only, as for any
   `Callable` value (FUN-05).
2. **Bind keyword arguments by the original function's parameter names**
   when the decorator returns the same `Callable` type. A wrapper whose
   parameters have other names would then be called with keywords it does
   not take.
3. **Carry the parameter names in the type with `**P`,** as
   `Callable[P, R]` does for type checkers. `ParamSpec` type parameters
   are planned for 0.13 (GEN-06).

## Decision

Option 1, for functions in 0.10.

- **Scope.** A decorator is any expression PEP 614 allows whose type
  can be called with the function, on a module-level or a nested
  function. Methods are planned for 0.14 (FUN-15).
- **Order.** When the `def` runs, the decorator expressions are evaluated
  top to bottom, then the defaults, and the decorators are applied bottom
  up, each call checked as a call with the previous result as its
  argument. The function itself is a value made for that call alone.
- **The name** is a variable of the outermost decorator's result type,
  module-level or local as the `def` is, bound when the `def` completes
  and read by the rules of any variable, so a read where a raising
  decorator may have left it unbound is refused or raises `NameError`
  as for any variable (TYP-12). Its result may be any type, a function
  value or not.
- **Calls** through the name are calls through its value: a keyword
  argument is E0404, since a `Callable` names no parameters, until
  `ParamSpec` carries the names (GEN-06).
- **Inside the body,** a read of the function's own name reads the
  variable, as in CPython. ADR-0061's rule that a nested function's own
  name reads the running function, and direct calls by name, apply only
  to functions without decorators.
- **Generic decorators,** such as `def trace[T, R](f: Callable[[T], R])
  -> Callable[[T], R]`, infer their type arguments from the function's
  type (ADR-0081). A generic function cannot be decorated, since it is no
  value (E0202).
- **Exports.** A decorated function of a library module is a variable, so
  it is no C export (E0429).
- **Classes.** A class decorator other than `@dataclass` whose type shows
  that it returns the class it is given is planned for 1.0 (CLS-38); one
  that may return anything else changes the class, which SRS 6.1
  excludes.
- **Method decorators** are told apart by name: only `@name.setter` of a
  property `name` the class defines is a setter.

## Consequences

- Wrappers, registries, factories such as `@repeat(3)`, memoizing
  decorators and generic decorators work on functions as in CPython.
- A call through a decorated name is an indirect call, and keyword
  arguments through it wait for `ParamSpec`, where CPython accepts the
  names its wrapper takes.
- Decorated methods, and their differences between CPython 3.12 and
  3.13, are left to FUN-15.

## Related requirements

FUN-06, FUN-05, FUN-15, CLS-38, GEN-06, TYP-12, ADR-0060, ADR-0061,
ADR-0081.
