# ADR-0087: Make typing values constants, written only where the oracles agree

## Status

Accepted

## Date

2026-11-20

## Context

TYP-18 makes the names of `typing` and `collections.abc` values that a
program stores, passes, compares and writes, and with them a type alias
a function or a class defines, an alias given its type arguments, and an
alias's `__value__` and `__type_params__`. Up to 0.9 TyPy reads these
names only in annotations, `isinstance` and class patterns; a type alias
the module defines is already a value, an immortal `TypeAliasObject`
written as its name, with `__name__`, `__module__` and `is`.

CPython 3.12.16, 3.13.18, 3.14.10 and 3.15.2, checked on 2026-11-20,
write some of these values differently:

| Expression | 3.12 and 3.13 | 3.14 | 3.15 |
| --- | --- | --- | --- |
| `Optional[int]` | `typing.Optional[int]` | `int \| None` | `int \| None` |
| `Union[int, str]` | `typing.Union[int, str]` | `int \| str` | `int \| str` |
| An alias a function `f` defines | `Local` | `Local` | `f.<locals>.Local` |
| `Pair[int]` | `Pair[int]` | `Pair[int]` | `__main__.Pair[int]` |

Other values are written alike by all four: `typing.Optional`,
`list[int]`, `typing.Iterator[int]`, `<class 'collections.abc.Iterator'>`,
`typing.Callable[[int], str]`, `int | None`, and an alias of a class such
as `type Score = int`, whose `__value__` is `<class 'int'>` and whose
`__type_params__` is `()`.

Equality is the same in every version: `Optional[int] == int | None` is
true and the two hash alike, `List[int] == list[int]` is false, and
`typing.Sequence == collections.abc.Sequence` is false. Identity is not
a rule of the language: `typing` caches `Iterator[int]`, so `is` holds
between two reads, while `collections.abc.Iterator[int]` and `int | None`
make a new object each time. `type(Optional[int])` is a private class
whose name changed in 3.14.

## Options considered

1. **Constants made when the program compiles.** Every typing value a
   program uses is written in its source, so each distinct one can be an
   immortal object holding its text, its equality key and its hash.
2. **Objects made when the program runs,** holding an origin and the
   arguments, as CPython's are. They would cost allocations and a runtime
   that formats and compares types, for values the compiler already
   knows.

## Decision

Option 1.

- **What a typing value is.** A name of `typing` or `collections.abc`
  read as a value; such a name, or a built-in class such as `list`, given
  type arguments; a union of types written with `|` outside an
  annotation; a type alias, wherever it is defined, and an alias given
  type arguments; an alias's `__value__` when it is not a class; and the
  type parameters an alias's `__type_params__` holds.
- **The object.** Each distinct typing value is one immortal object, as a
  module's type alias already is, holding its text, its qualified name,
  and a key that equal values share, such as one key for `Optional[int]`
  and `int | None`. A type alias a function defines is the exception: as
  in CPython, its `type` statement makes a new alias each time it runs,
  an object that refers to the alias's immortal description.
- **Its type.** A type alias keeps the type `TypeAliasType` that a
  module's alias has had since 0.4, whose `type()` is
  `typing.TypeAliasType`. Every other typing value has one static type,
  written "typing value" in messages. A variable, a list, tuple or dict
  display, a dict key, a set item and a type parameter of a generic
  function take these types as any other; no annotation spells them in
  0.10. `__value__` of an alias of a class is that class's type object,
  and `__type_params__` is a tuple of typing values, `()` for an alias
  without type parameters.
- **Equality and hash** go by the key, giving CPython's answers. An alias
  compares by identity, as CPython's `TypeAliasType` does, and an alias
  given type arguments by its alias and its arguments, so `Pair[int] ==
  Pair[int]` is true. A hash's number differs between runs of CPython, so
  no test prints it.
- **Text.** A typing value is written as CPython 3.15 writes it
  (ADR-0029). Writing one that an oracle from the program's Python floor
  writes otherwise, with `print`, `str`, `repr` or an f-string, is E0202,
  naming the floor that accepts it (ADR-0008): 3.14 for a union written
  from `Optional` or `Union`, and 3.15 for an alias a function or a class
  defines, an alias given type arguments, and a `__value__` that names a
  class of the program or the alias itself. Storing, passing, comparing
  and hashing such a value is accepted from any floor.
- **Identity.** `is` between two bare names, or between two aliases, is
  accepted, since each is one object in CPython too; between other typing
  values it is E0202, since CPython caches some and not others.
- **Later.** An annotation for typing values (`types.GenericAlias`,
  `types.UnionType`, `typing.TypeAliasType`), calling one such as
  `list[int]()`, `isinstance` with one held in a variable, `type()` of
  one, `__origin__`, `__args__`, `__name__`, `typing.get_origin` and
  `typing.get_args`, the attributes of a type parameter, and an alias a
  function defines given type arguments, are planned for 1.0 (TYP-22).

## Consequences

- `print(list[int])`, `kinds = [Optional[int], int | str]` and
  `Optional[int] == int | None` run as in CPython, at no cost when the
  program runs beyond reading a constant.
- At the default floor 3.12, most written forms of `Optional`, `Union`
  and local aliases are refused, because the oracles disagree; a program
  that needs them sets its floor.
- A function cannot yet declare a parameter or result of typing values,
  only take one through a type parameter, until TYP-22.

## Related requirements

TYP-18, TYP-22, TYP-17, GEN-01, ADR-0008, ADR-0023, ADR-0029.
