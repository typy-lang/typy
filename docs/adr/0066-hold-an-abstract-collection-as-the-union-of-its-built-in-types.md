# ADR-0066: Hold an abstract collection as the union of its built-in types

## Status

Accepted

## Date

2026-11-19

## Context

`Sequence`, `Mapping` and the other abstract collections of
`collections.abc` and `typing` are the types a Python function names
when it takes any list or tuple, or any dict-like value (LIB-20). CPython
registers its built-in types with them, checked with CPython 3.12.16 on
2026-11-19:

| Abstract collection | Built-in types registered with it |
| --- | --- |
| `Sequence` | `list`, `tuple`, `str`, `bytes`, `range` |
| `MutableSequence` | `list` |
| `Mapping`, `MutableMapping` | `dict` |
| `Set` | `set`, `frozenset`, the keys and items views of a dict |
| `MutableSet` | `set` |
| `Collection` | each of the above, and the values view of a dict |

TyPy has no `frozenset` and no `range` value, so neither can be passed
where one of these is expected. A value of an abstract collection type
must be held in one word, and its operations, such as `len`, an index, a
loop and `in`, must work for each type it may hold. TyPy already checks
an operation on a union, such as `list[int] | tuple[int, ...]`, once for
each member, and the runtime tells the members apart by their kind
(TYP-14, ADR-0027).

## Options considered

1. **The union of the built-in types registered with it,** with their
   item types: `Sequence[int]` holds a `list[int]`, a `tuple[int, ...]`
   or `bytes`, and each operation is checked member by member.
2. **A protocol object for each collection,** a table of `len`, item,
   loop and `in` functions that each value carries, as ADR-0065 gives
   iterators. It would also serve classes of the program, but every
   operation would become an indirect call, and a class that derives from
   an abstract collection inherits CPython's mixin methods, which TyPy
   would have to provide.
3. **A copy of each function for each type it receives,** as generic
   functions have (ADR-0023). A list of sequences of several types could
   then not exist.

## Decision

Option 1.

- **The members.** `Sequence[T]` is `list[T]`, `tuple[T, ...]`, `str` when
  `T` is `str`, and `bytes` when `T` is `int`; `MutableSequence[T]` is
  `list[T]`; `Mapping[K, V]` and `MutableMapping[K, V]` are `dict[K, V]`;
  `Set[T]` is `set[T]`, the keys view of a `dict[T, V]` and the items view
  of a `dict` whose items are `T`; `MutableSet[T]` is `set[T]`; and
  `Collection[T]` is each of these, and the values view of a `dict` of
  values `T`. `typing.AbstractSet` is `Set`.
- **Operations** are those of the abstract collection, each checked for
  every member and run on the member the value holds: an operation that
  only some members have, such as `append` on a `Sequence`, is E0412, as
  type checkers report it.
- **Messages and `repr` of types** name the abstract collection, such as
  `Sequence[int]`, not its members.
- **`isinstance`** with an abstract collection answers as CPython does
  for each member.
- **A class of the program that derives from an abstract collection**,
  with the mixin methods it inherits, is planned for 1.0 (CLS-37).

## Consequences

- A function that takes a `Sequence[int]` accepts a list, a tuple and
  bytes, as under CPython, with no new kind of object.
- Each operation on such a value tests the member it holds, as an
  operation on a union does; an abstract collection with many members,
  such as `Collection`, gives each operation as many branches.
- A `Sequence[T]` is no `Iterator[T]`, and a loop over it takes the loop
  of its member.
- `frozenset` and `range` join the members when TyPy gives them values.

## Related requirements

LIB-20, TYP-14, CLS-37, ADR-0023, ADR-0027, ADR-0065.
