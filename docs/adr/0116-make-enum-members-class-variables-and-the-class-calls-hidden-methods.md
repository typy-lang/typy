# ADR-0116: Make enum members class variables, and the class's calls hidden methods

## Status

Accepted

## Date

2026-11-23

## Context

ADR-0093 expands the classes of `enum` in the checker, as dataclasses
and named tuples are expanded. CPython's `EnumType` metaclass gives an
enum class what no other class has: its members are instances made when
the class statement runs, a call of the class finds a member by its
value rather than making an instance, a subscript finds one by its name,
and the class itself is iterable and has a length. TyPy compiles a class
statically: a class variable is a module variable `Class.name` given its
value where the class statement runs (ADR-0024), and a call of a class
makes an instance through `__init__`.

## Options considered

1. **Give enum classes a kind of their own** in the checker, the typed
   tree, the lowering and the runtime: member objects in a static table,
   and operations on the type object for the call, the subscript and
   iteration.
2. **Write the enum class as an ordinary class,** whose members are class
   variables, each a new instance made through a written `__init__`, with
   written methods for what `EnumType` does, and have the checker call
   those methods where the program uses the class as a value.

## Decision

Option 2. The expansion keeps the class's own methods and writes:

- one class variable for each member, in the order the body defines them:
  a call of a hidden name, `" (member) Color"`, which the checker reads as
  `Color.__init__` with the member's name and value, and for an alias,
  the variable of the member it names, so `Color.CRIMSON is Color.RED`;
- `__init__`, the `name` and `value` properties, `__repr__` and
  `__str__`, as `Enum` gives them;
- three hidden static methods, whose names hold a space so no program
  writes them: `" (lookup)"` returns the member whose value equals its
  argument or raises CPython's `ValueError`, `" (named)"` the member a
  name names or `KeyError`, and `" (members)"` a new list of the members
  without aliases.

The checker calls `" (lookup)"` for `Color(value)`, `" (named)"` for
`Color[name]`, and `" (members)"` where a loop, `len` or `in` reads the
class. A member's value is an `int`, `str`, `bytes` or `bool` constant or
`auto()`, and `value` has the union of their types. `auto()` gives one
more than the largest value before it, or `1`; after a value that is not
an `int` it is refused, since CPython 3.12 gives `1` with a warning and
3.13 raises `TypeError`.

## Consequences

- No new kind of class, typed expression or runtime object: the members,
  their methods and the lookups compile as the program's own classes and
  functions do, and lowering is unchanged.
- A call of the class takes a value of the members' value type, where
  CPython takes any value and raises `ValueError` for one no member has;
  a value of another type is E0404.
- Members of other values, annotated members, `__init__`, `__new__`,
  `_missing_`, `_generate_next_value_`, other bases, decorators such as
  `@unique`, and classes derived from enum classes without members are
  planned for 0.13 (CLS-47).

## Related requirements

CLS-44, CLS-47, CLS-11, CLS-08, ADR-0093, ADR-0024, ADR-0088.
