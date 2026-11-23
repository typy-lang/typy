# ADR-0117: Read an int or str enum member as its value where the value is wanted

## Status

Accepted

## Date

2026-11-23

## Context

`IntEnum` and `StrEnum` derive from `int` and `str` in CPython, so their
members are numbers and texts too: `Level.HIGH + 1`, `[1, 2][Level.LOW]`,
`Mode.READ.upper()` and a member passed for an `int` parameter all work,
while `repr` shows `<Level.HIGH: 10>`. TyPy holds an `int` as a machine
word and a `str` as a text object, and the expansion of ADR-0116 makes
each member an instance of the program's own class, which is neither.

## Options considered

1. **Make a member an `int` or a `str` value** in the program, with its
   class known only to the checker. `repr`, `name`, `type()` and
   `isinstance` would need the member back from the value.
2. **Keep a member an instance, and give its class every method of `int`
   or `str`** as written source. Each operator, method and builtin needs
   a method of its own, and a member passed for an `int` still fails.
3. **Keep a member an instance, and have the checker read its value where
   the program uses it as a number or a text:** an operand of an
   arithmetic operator or a comparison, an index, an argument of the
   builtins of numbers and text, a value where an `int` or a `str` is
   declared, the receiver of a method its class does not have, and a
   field of an f-string with a format specification.

## Decision

Option 3. The expansion gives an `IntEnum` or a `StrEnum` the hidden
method `" (value)"`, which marks the class, with `__str__` and `__hash__`
of the value, `__bool__` of the value's truth, and `__lt__`, `__le__`,
`__gt__` and `__ge__` between members, which `sorted` and `max` use.
`enum_value` reads a member of such a class as the attribute `_value_`
at each place option 3 lists. `is`, `is not`, and `in` over a container
of members keep the member.

## Consequences

- A member works as a number or a text in the places programs use one,
  and stays a member for `repr`, `name`, `value`, `type()`,
  `isinstance`, `match` and as a key, with no change to lowering or the
  runtime.
- A container of members is not a container of their values: a
  `list[Mode]` passed for an `Iterable[str]`, as in
  `", ".join(list(Mode))`, is E0404, which `[m.value for m in Mode]`
  avoids.
- A place not listed, such as `"%d" % Level.LOW`, refuses a member with
  E0427 or E0404, and `Level.LOW.value` passes the value there.

## Related requirements

CLS-44, ADR-0116, ADR-0093.
