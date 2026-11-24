# ADR-0110: Hold a literal type's values as its base type

## Status

Accepted

## Date

2026-11-23

## Context

TYP-28 accepts `typing.Literal` of `int`, `str`, `bytes`, `bool` and
`None` constants: a constant passes for its literal type, and `==`, `in`
and `match` narrow a variable to the constants it is compared with. A
value of `Literal['r', 'w']` is a `str` when the program runs, so the
question is where the checker keeps the literal type and how the code that
lowers the program sees it.

The lowering crate matches on the checked type of every expression, more
than 900 times, to pick the runtime operation. A literal type in an
expression's type would need an arm in each of those places, or a pass
that rewrites every typed expression.

The abstract collections of ADR-0086 had the same shape: a declared type
that is wider than any value's type. There, an expression's type is never
abstract, `BodyChecker::typed` widens it, and the declared type stays where
it is written.

## Options considered

1. **A literal type in expression types,** with every lowering match taught
   to read it as its base type. It is the most precise, and it touches
   every lowering file.
2. **A literal type only in declared types:** parameters, results,
   variables, attributes and type arguments. An expression's type is its
   base type, as `BodyChecker::typed` makes it. Where a value meets a
   declared literal type, the checker reads what the value is from the
   value itself: a constant, a variable or attribute declared or narrowed
   to a literal type, or a call whose result is declared one. The typed
   program given to lowering has no literal type.
3. **Literal types as a table beside the types,** keyed by parameter,
   variable and attribute. It loses literal types inside other types, such
   as `list[Literal['r']]`.

## Decision

Option 2. `Type::Literal` holds one `LiteralValue`; `Literal['r', 'w']` is
the union of two. `expression_expecting` tests a value against a declared
literal type with `takes_literals_of`, and refuses another with E0431. A
default value is tested where the definition is read. A comparison of a
variable with constants records a `LiteralTest`, whose facts narrow the
variable as an `isinstance` test does. `Program::finish` gives the
symbols, signatures and attributes their base types.

A narrowing never changes how a value is held, except to take the value
out of an optional once a test shows it is not `None`. A variable of a
`str` narrows to `Literal['r']`, and one of `Literal['r', 'w']` to
`Literal['w']` when `== 'r'` fails; a variable of `Literal['r', 1]` is held
as `str | int` and is not narrowed by `==`, so a wrong static type can never
reach a value held in another form.

## Consequences

- Lowering, the runtime and the generated code do not change: a literal
  type costs nothing when the program runs.
- `is_assignable_to` lets a value of the base type pass where a literal type
  is declared, since the value test happens where the value is written.
  Two declared types compare by base type where they are not both literal
  types, so an override whose result is `str` passes for a base method's
  `Literal['r']` result, and `list[str]` for `list[Literal['r']]`. The
  value is a `str` either way, so the program behaves as CPython runs it.
- A literal type in a library's exported signature is written as its base
  type.
- `match` over a variable of literal types covers it with a case for each
  value, so a function that returns from each case needs no return after
  the `match`.

## Related requirements

TYP-28, TYP-05, TYP-07, CST-07, ADR-0086.
