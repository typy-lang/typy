# ADR-0085: Hold a named tuple as an instance of a class derived from `tuple`

## Status

Accepted

## Date

2026-11-20

## Context

`class Point(NamedTuple)` with annotated fields declares a tuple whose
items have names (CLS-27). CPython 3.12.16 to 3.15.2, checked on
2026-11-20, make it a class derived from `tuple`:

- `Point(1, 2)`, `Point(x=3, y=4)` and `Point(5)` with a default build
  it; `repr` is `Point(x=1, y=2)`; `p.x`, `p[0]`, `p[-1]`, unpacking,
  `len`, `in`, `count`, `index`, `list(p)` and `tuple(p)` work, and a
  slice, `p + (3,)` and `p * 2` give plain tuples.
- `p == (1, 2)` is true, as is `Point(1, 2) == Other(1, 2)` for another
  named tuple; `p < (1, 3)` orders as tuples do, and `hash(p) ==
  hash((1, 2))`.
- `isinstance(p, tuple)` is true and `type(p).__mro__` is `(Point, tuple,
  object)`. `_fields`, `_field_defaults`, `__match_args__`, `_replace`,
  `_asdict` and `_make` exist, and `p.x = 5` raises `AttributeError:
  can't set attribute`.
- The body may hold methods, properties, class methods, static methods
  and class variables, and a subclass may add methods. A mutable default
  is one object every instance shares.
- `class Pair[T](NamedTuple)` is generic.
- When the class statement runs, a field without a default after one
  with a default, a field name that starts with an underscore, a member
  named `_replace`, `__init__` or `__new__`, and a second base raise
  `TypeError`, `ValueError` or `AttributeError` with fixed texts.
- `p._replace(z=9)` raises `ValueError` in 3.12 and `TypeError` from
  3.13, and the form `NamedTuple("P", [("x", int)])` behaves differently
  in 3.15 for duplicate names.

## Options considered

1. **A fixed tuple, held as a C structure,** with the field names known
   when the program compiles. It allocates nothing, but a value has no
   class: a `Point` held as a `Point` base class, or in a
   `list[tuple[int, int]]`, would print `(1, 2)` and give `tuple` as its
   type, so subclasses and those conversions would be refused.
2. **An instance of a class whose type object derives from the runtime's
   `tuple`,** its fields laid out as a tuple of the same item types.
3. **A fixed tuple with a word for its class.** It keeps the class without
   an allocation, but every container would need a new kind of item, and
   a plain tuple holding a `Point` would still lose the word.

## Decision

Option 2.

- **The value.** A named tuple is a heap object whose type object's base
  is the runtime's `tuple` type object. Its fields follow the header in
  the order and alignment of a tuple of their types (ADR-0062), so the
  runtime's tuple functions give its hash, its equality and order with
  plain tuples and other named tuples, field by field, and its items. An
  instance that does not escape its function is allocated in the frame
  (ADR-0042).
- **The class.** The compiler writes `__new__` from the fields and
  defaults, `__repr__`, `_fields`, `_field_defaults`, `__match_args__`,
  `_replace`, `_asdict` and `_make`. Defaults are evaluated once, when the
  class statement runs, and shared. `_asdict` and `_field_defaults` give a
  `dict[str, U]` of the union `U` of the field types.
- **Members.** The body may hold methods, properties, class methods,
  static methods, class variables, `__repr__`, `__str__`, `__eq__`,
  `__hash__` and the order methods, and a field may hide a tuple method
  of its name, such as `count`. A class with `__eq__` compares as
  ADR-0038 says, which calls it first also when a plain tuple is on the
  left.
- **Tuple operations.** An index by literal gives the field's type; a
  computed index, a loop and `in` follow the rules of a fixed tuple, so
  fields of different types need a common item type (E0307, E0411). A
  slice, `+` and `*` give plain tuples, and `tuple(p)` and `list(p)` give
  plain values. Iteration entries (ADR-0065) let a named tuple pass as an
  `Iterable[T]`.
- **No silent conversion.** A named tuple where a plain tuple type or an
  abstract collection is expected is refused, since the plain value would
  lose its class, and planned for 1.0 (CLS-42).
- **Errors.** Each error CPython raises when the class statement runs is
  refused when the program compiles, with CPython's text; so are a field
  assignment, a call with arguments that do not fit, and `_replace` with a
  name that is not a field.
- **Scope.** The class form; a generic one, such as
  `class Pair[T](NamedTuple)`, copied as any generic class (ADR-0023);
  class patterns and sequence patterns; and subclasses that add methods.
  `Generic[T]` as a second base stays refused, as `Generic` is (GEN-01).
  The form made by a call, a subclass with `__new__`, `__init__` or
  attributes of its own, and a body that defines `__len__`,
  `__getitem__`, `__iter__` or `__contains__` are planned for 1.0
  (CLS-41).

## Consequences

- A named tuple prints, compares, hashes and tests with `isinstance` as
  in CPython, and its class takes methods and subclasses.
- Each named tuple is one allocation and one reference count, where a
  plain tuple is held in place; escape analysis removes the allocation
  for a value that stays in its function.
- `p` passed where a `tuple[int, int]` is expected is refused until 1.0;
  `tuple(p)` is the replacement.

## Related requirements

CLS-27, CLS-41, CLS-42, CLS-04, GEN-01, ADR-0012, ADR-0021, ADR-0023,
ADR-0038, ADR-0042, ADR-0062, ADR-0065.
