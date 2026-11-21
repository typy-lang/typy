# ADR-0086: Hold an abstract collection as its built-in types, `range` included

## Status

Accepted

## Date

2026-11-20

## Context

ADR-0066 holds `Sequence`, `Mapping` and the other abstract collections
of `collections.abc` and `typing` as the union of the built-in types
CPython registers with them (LIB-20). It was written believing TyPy had
no `range` value, but `range` has been a value since 0.3 (DAT-17), so its
list of members is short of one. It also leaves out names that programs
import beside `Sequence`, `Container`, `Sized`, `Reversible`, `Hashable`
and the views of a dict, says that `typing.AbstractSet` is `Set` while
the import is refused with E0425, and plans CLS-37 for 1.0 where the SRS
now plans it for 0.14.

CPython 3.12.16 to 3.15.2, checked on 2026-11-20, register these built-in
types that TyPy has:

| Abstract collection | Built-in types of TyPy registered with it |
| --- | --- |
| `Sequence` | `list`, `tuple`, `str`, `bytes`, `range` |
| `MutableSequence` | `list` |
| `Mapping`, `MutableMapping` | `dict` |
| `Set`, `typing.AbstractSet` | `set`, the keys and items views of a dict |
| `MutableSet` | `set` |
| `Collection`, `Container` | each of the above, and the values view |
| `Reversible` | the sequences, `dict` and the three views |

`Sequence`, `Mapping` and `Set` are nominal: a class of the program with
`__len__` and `__getitem__` is no `Sequence`. `Collection`, `Container`,
`Sized`, `Reversible` and `Hashable` are structural: a class with the
methods is one, for `isinstance` and for type checkers. `Sized` and
`Hashable` take no item type, and `Hashable` holds `int`, `float` and
`None`, which TyPy holds without a header.

A slice of a `Sequence` keeps its member's type, `range(1, 3)` for a
range; `[1, 2] + (3,)` raises `TypeError`. Type checkers accept a
`list[bool]` and a `list[Square]` where a `Sequence[int]` and a
`Sequence[Shape]` are expected.

## Options considered

1. **Keep ADR-0066's union of built-in types,** with `range`, the names
   it left open, and the rules it did not state.
2. **A protocol object for each collection,** which classes of the
   program could also fill. ADR-0066 rejected it for the cost of an
   indirect call on every operation.

## Decision

Option 1. This record supersedes ADR-0066.

- **Members.** `Sequence[T]` holds `list[T]`, `tuple[T, ...]`, `str` when
  `T` is `str`, and `bytes` and `range` when `T` is `int`.
  `MutableSequence[T]` holds `list[T]`. `Mapping[K, V]` and
  `MutableMapping[K, V]` hold `dict[K, V]`. `Set[T]`, and
  `typing.AbstractSet[T]` with it, hold `set[T]`, the keys view of a
  `dict[T, V]` and the items view of a `dict` whose items are `T`.
  `MutableSet[T]` holds `set[T]`. `Collection[T]` and `Container[T]` hold
  each of these and the values view of a `dict` of values `T`.
  `Reversible[T]` holds the sequences, `dict[T, V]` and the three views.
  `KeysView[K]`, `ValuesView[V]` and `ItemsView[K, V]` are the views.
- **Later.** `Sized`, `Hashable` and `MappingView`, which take no item
  type, and classes of the program with the methods of `Collection`,
  `Container` or `Reversible`, are planned for 0.14 (LIB-24), as are
  classes derived from an abstract collection (CLS-37).
- **Operations** are those of the abstract collection, checked for every
  member and run on the member the value holds, as for a union (ADR-0027).
  An operation only some members have, such as `append` on a `Sequence`,
  is E0412. A slice gives the abstract collection of the slice's members,
  and `+` between two values of `Sequence[T]` is E0401, since the members
  may differ.
- **Covariance.** A collection of instances of a class derived from the
  item class passes unchanged, since an instance is one pointer whatever
  its class: a `list[Square]` is a `Sequence[Shape]`. A collection whose
  items would change how they are held, such as a `list[bool]` for a
  `Sequence[int]`, is E0404.
- **Messages and the text of types** name the abstract collection, such
  as `Sequence[int]`.
- **`isinstance`** with an abstract collection answers as CPython does for
  each member, narrows a union to its members that are instances, and
  is false for instances of the program's classes until CLS-37 and
  LIB-24.

## Consequences

- A function that takes a `Sequence[int]` accepts a list, a tuple, bytes
  and a range, as under CPython, with no new kind of object.
- Each operation on such a value tests the member it holds; `Collection`
  has up to eight members, so an operation on it has up to eight
  branches, which the benchmark suite measures (NFR-21).
- A `list[bool]` passed as a `Sequence[int]` is refused where type
  checkers accept it, as TyPy refuses an `int` where a `float` is declared
  (TYP-09).

## Related requirements

LIB-20, LIB-24, CLS-37, DAT-17, TYP-09, TYP-14, NFR-21, ADR-0023,
ADR-0027, ADR-0065, ADR-0066.
