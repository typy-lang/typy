# ADR-0088: Give a named tuple methods over the tuple of its fields

## Status

Accepted

## Date

2026-11-21

## Context

ADR-0085 makes a named tuple an instance of a class whose type object
derives from the runtime's `tuple`, and has the runtime's tuple
functions hash, compare and order it, reading its fields as a tuple's
items (CLS-27). Building it on 2026-11-21 showed three things:

- A fixed tuple is not an object in TyPy. It is held in place, and the
  compiler emits its comparisons, hashing and items for its type; the
  runtime reads one only as an item of a container, through the
  container's tuple descriptor (ADR-0009, ADR-0062). The runtime has no
  function that compares an object with a tuple held in place, so
  `p == (1, 2)` would need new runtime entry points and a second path
  through the type object's slots.
- The dataclass expansion already writes `__init__`, `__repr__`,
  `__eq__`, `__hash__` and the order methods of a class from its fields,
  as source the checker reads (CLS-05). Each of them over the tuple of
  the fields gives what CPython's `tuple` gives: `hash((1, 2))`, the
  comparison item by item, and the order of the first unequal item.
- An instance's attributes are laid out by the rule of ADR-0062, the
  rule of a tuple's items, so a named tuple's memory is a tuple's after
  the header and the class.

CPython 3.12.16 to 3.15.2 behave as ADR-0085 records.

## Options considered

1. **ADR-0085 as written.** The type object's slots call the runtime's
   tuple functions with the class's descriptor, and the runtime gains
   functions that compare and order an object with a tuple held in place.
2. **Methods over the tuple of the fields.** The compiler writes the
   methods as it writes a dataclass's, each over the tuple of the
   fields, and a hidden method that returns that tuple. Where a tuple
   operation applies to a named tuple, the checker calls the hidden
   method and applies the operation to the tuple it returns. The type
   object names the runtime's `tuple` as its base.
3. **A named tuple as a fixed tuple with a class word,** which ADR-0085
   already set aside.

## Decision

Option 2. Each other part of ADR-0085's decision stands, restated here.

- **The value.** A named tuple is an instance of its class. Its type
  object's base is the runtime's `tuple` type object, so `isinstance(p,
  tuple)` is true when the program runs, and the checker knows it is
  true for a value whose type is the class. An instance that does not
  escape its function is allocated in the frame (ADR-0042).
- **The class.** The compiler writes `__init__` from the fields and
  their defaults, `__repr__`, `__eq__`, `__hash__`, `__match_args__`,
  `_fields`, and the order methods when every field is a number, a
  string, `bytes`, or a list or tuple of them; each compares or hashes
  the tuple of the fields. It writes `_asdict`, which gives a
  `dict[str, U]` of the union `U` of the field types, and for the class
  and each class derived from it a hidden method `_replace` calls. That
  method takes each field as `tuple[F] | None`, `None` for a field the
  call leaves as it is, so a field that holds `None` is told apart from
  one left out, and it makes an instance of its own class, so `_replace`
  on an instance of a derived class gives one of that class. Defaults
  are constants, as for any function (EXP-20).
- **Tuple operations.** An index, a slice, `len`, `in`, a loop,
  unpacking, `*` in a call, a sequence pattern, `+` and a comparison
  with a tuple or another named tuple read the named tuple as the tuple
  of its fields, through the hidden method, so the value is evaluated
  once. Each follows the rules of a fixed tuple: an index is an integer
  literal, a loop needs items of one type (E0307, E0411), and the
  methods of `tuple` come with those of a fixed tuple. A
  method the class defines for an operator, such as `__eq__` or
  `__add__`, is called first, as ADR-0026 and ADR-0038 say.
- **Members.** The body may hold methods, properties, class methods,
  static methods, class variables, `__repr__`, `__str__`, `__eq__`,
  `__hash__` and the order methods. A class that defines `__eq__` and
  not `__hash__` has no hash, as in CPython.
- **No silent conversion.** A named tuple where a plain tuple type or an
  abstract collection is expected is refused, and planned for 1.0
  (CLS-42).
- **Errors.** Each error CPython raises when the class statement runs is
  refused when the program compiles, with CPython's text and the name of
  the exception; so are a field assignment, with CPython's `AttributeError:
  can't set attribute`, a call with arguments that do not fit, and
  `_replace` with a name that is not a field.
- **Scope.** The class form, a generic one such as `class
  Pair[T](NamedTuple)`, class and sequence patterns, and classes derived
  from a named tuple that add methods. The form made by a call, a class
  derived from a named tuple with `__new__`, `__init__` or attributes of
  its own, and a body that defines `__len__`, `__getitem__`, `__iter__`
  or `__contains__` are planned for 1.0 (CLS-41). `_make`,
  `_field_defaults`, and `_replace` read as a value, called through the
  class or given `**`, are planned for 0.14 (CLS-43): `_make` takes any
  iterable, whose items a named tuple of fields of several types could
  not take without a cast.

## Consequences

- A named tuple prints, compares, hashes and tests with `isinstance` as
  in CPython, and the runtime gains no function for it.
- `sorted` of a list of named tuples whose fields are of other types,
  such as another named tuple, is refused, where CPython orders them; a
  comparison the program writes between two of them is accepted when
  the fields order.
- A comparison between a named tuple and a tuple held in place is
  emitted as one between two tuples, so it costs a tuple's comparison.
- ADR-0085 is superseded.

## Related requirements

CLS-27, CLS-41, CLS-42, CLS-43, CLS-05, CLS-04, EXP-20, ADR-0009, ADR-0026,
ADR-0038, ADR-0042, ADR-0062, ADR-0085.
