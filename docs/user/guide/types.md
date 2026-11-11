# Types and annotations

Every value in a TyPy program has one type, fixed when the program is
compiled. Functions are annotated; variables take the type of their first
value.

## What TyPy 0.2 accepts

| Item | Rule |
| --- | --- |
| Annotations on every function parameter, except `self`, and on every return | TYP-01 |
| A variable without an annotation takes the type of its first assignment | TYP-02 |
| Assigning a value of another type to a variable is refused | TYP-03 |
| Inference stays inside one function | TYP-04 |
| An empty `[]`, `{}`, `set()` or `dict()` needs an annotation or a typed context | TYP-05 |
| `list[T]`, `dict[K, V]`, `set[T]`, `tuple[A, B]`, `tuple[T, ...]` and `T \| None` | TYP-06 |
| An `int` is refused where a `float` is declared, since CPython keeps it an `int` | TYP-09 |
| `typing.Any`, `typing.cast` and `object` annotations are refused | TYP-10 |
| A variable read before it is assigned on every path is refused | TYP-12 |
| `T \| None` narrows to `T` after `is not None`, a failed `is None` test or a truth test | TYP-13 |
| `Iterator[T]` and `Iterable[T]` from `typing` or `collections.abc` | TYP-15 |
| `TracebackType` and `ModuleType` from `types` | CST-10, MOD-10 |

Unions of other types, such as `int | str`, come with `isinstance` in 0.4
(TYP-14).

## Iterators and iterables

`Iterable[T]` takes any built-in iterable of items of type `T`: a list, a
`tuple[T, ...]` or a tuple whose items all have type `T`, a set, a dict,
whose items are its keys, a view of a dict, a `str` for `T` of `str`,
`bytes` or `range` for `int`, and an iterator. The value is the object
itself, so a list passed as an iterable can be read twice, as in CPython.
`Iterator[T]` takes an iterator only:

<!-- typy-expect: accepted -->
```python
from collections.abc import Iterable, Iterator


def total(values: Iterable[int]) -> int:
    return sum(values) + sum(values)


def countdown(start: int) -> Iterator[int]:
    return iter(range(start, 0, -1))


print(total([1, 2]), total(range(3)), total({4: "x"}), list(countdown(3)))
```

An iterable may be read by `for`, `iter`, a comprehension and the
builtins that read an iterable, such as `sum` and `sorted`. Its length and
its truth depend on which collection it is, so `len`, `bool`, `not` and a
condition on it are refused. In 0.3 these names, and `typing`,
`collections.abc` and `types` themselves, are for annotations only; their
use as values, and the other names of `typing`, come in 0.4 (LIB-06).

## Example

<!-- typy-expect: accepted -->
```python
def find(names: list[str], wanted: str) -> int | None:
    for index, name in enumerate(names):
        if name == wanted:
            return index
    return None


scores: dict[str, list[float]] = {}
scores["ada"] = [9.5, 8.0]
position = find(["ada", "alan"], "alan")
if position is not None:
    print("found at", position + 1)
print(scores, find([], "x"))
```

An optional value must be tested before it is used as its base type:

<!-- typy-expect: rejected E0401 at 2:7 -->
```python
limit: int | None = None
print(limit + 1)
```
