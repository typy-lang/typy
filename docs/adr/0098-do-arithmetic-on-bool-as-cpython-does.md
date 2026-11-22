# ADR-0098: Do arithmetic on bool as CPython does

## Status

Accepted

## Date

2026-11-21

## Context

DAT-03 rejected arithmetic on `bool` operands since 0.1: `True + 1` was
E0406, and so were `-True`, `abs(True)`, `round(True)` and `sum` of a
list of bools. CPython defines each of them: a `bool` is an `int`, 0 or 1,
so `True + 1 == 2`, `True / 2 == 0.5`, `-True == -1`, and `sum(flags)`
counts the true items. The rule TyPy keeps is to accept valid Python whose
result is defined, and to reject only what CPython leaves undefined or
words differently by version.

`object` (TYP-23, ADR-0090) made the rejection a real obstacle:
`isinstance(x, int)` holds for `True`, so it narrows an `object` to `int |
bool`, which keeps `print(x)` printing `True`, and `total += x` after the
test was refused.

`~True` is the exception: CPython 3.12 to 3.15 print a
`DeprecationWarning` for it, which says it is removed in Python 3.16.

## Options considered

1. **Keep rejecting arithmetic on `bool`.** A pure `int` would need
   `type(x) is int`, and common Python would be refused.
2. **Narrow `isinstance(x, int)` to `int` alone.** A `bool` would unbox to
   0 or 1, so `print(x)` would print `1` where CPython prints `True`, which
   STD-04 forbids.
3. **Do arithmetic on `bool` as CPython does.**

## Decision

Option 3, chosen by the project on 2026-11-21.

- Beside a number, a `bool` operand of `+`, `-`, `*`, `/`, `//`, `%`, `**`,
  `<<`, `>>`, `&`, `|` and `^` is the `int` it is; `&`, `|` and `^` of two
  bools keep giving a `bool` (DAT-03). A `True` or `False` written in the
  program is the constant 1 or 0, so `2 ** True` has a literal exponent.
- Beside a sequence, `*` takes a `bool` as the repeat count, as in `"ab" *
  True`.
- Unary `-` and `+` give the `int`, and `abs`, `round` and `sum` take bools
  as ints; `sum` of a list of bools starts from an `int` and counts the
  true items (`typy_rt_list_sum_bool`).
- `==`, `!=` and the orderings compare a `bool` with an `int` or a `float`
  as its `int`, so `True == 1` holds.
- An `int | bool` value an `isinstance` test gave combines member by
  member, also in `total += value`.
- `~` of a `bool` is E0406.

## Consequences

- Each of these programs runs with CPython's result, and the `object`
  idiom `if isinstance(value, int): total += value` compiles.
- A variable declared `bool` still holds only a `bool`: `flag += 1` gives
  an `int`, which TYP-03 refuses to assign to it.
- E0406 now means `~` of a `bool` only.

## Related requirements

DAT-19, DAT-03, TYP-23, STD-04, ADR-0090.
