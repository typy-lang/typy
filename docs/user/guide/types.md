# Types and annotations

Every value in a TyPy program has one type, fixed when the program is
compiled. Functions are annotated; variables take the type of their first
value.

## What TyPy accepts

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
| A variable narrows to a derived class after `isinstance` | TYP-07 |
| A name, class variable or attribute declared `Final` is not assigned again | TYP-08 |
| `type Name = annotation` at module level names an annotation | SMT-10 |
| `A \| B`, such as `int \| str`, narrowed by `isinstance` | TYP-14, TYP-07 |
| `Iterator[T]` and `Iterable[T]` from `typing` or `collections.abc` | TYP-15 |
| `TracebackType` and `ModuleType` from `types` | CST-10, MOD-10 |

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

## Unions

`A | B` holds a value of either type, such as `int | str`,
`str | bytes`, `list[int] | dict[str, int]` or two classes. `A | B` and
`B | A` are one type, and a member another member takes is dropped:
`Circle | Shape` is `Shape` when `Circle` derives from `Shape`. A union
of objects is held as the object itself; a union with a number, a `bool`
or a tuple among its members is held with a tag that says which member it
holds, so storing an `int` in an `int | str` allocates nothing
(ADR-0027).

A union can be passed, returned, stored in a container, printed, written
with `str`, `repr` and f-strings, given to `type()` and tested for truth.
`isinstance` narrows it, in the branch where the test holds and in the
branch where it fails. A union of classes that share a base has that
base's attributes and methods:

<!-- typy-expect: accepted -->
```python
class Shape:
    def __init__(self, name: str) -> None:
        self.name = name


class Circle(Shape):
    def __init__(self, radius: float) -> None:
        super().__init__("circle")
        self.radius = radius


class Square(Shape):
    def __init__(self, side: float) -> None:
        super().__init__("square")
        self.side = side


def size(value: str | list[int]) -> int:
    if isinstance(value, str):
        return len(value)
    return sum(value)


def measure(shape: Circle | Square) -> str:
    if isinstance(shape, Circle):
        return shape.name + " " + str(shape.radius)
    return shape.name + " " + str(shape.side)


print(size("abc"), size([1, 2]), measure(Square(2.0)))
```

Any other operation on a variable that holds a union, such as a method,
an attribute, an operator, an item or `len`, is checked once for each
member, and the program runs the one for the member the object is. Each
member must allow it, so an error names the member that does not:

<!-- typy-expect: accepted -->
```python
def describe(value: str | bytes) -> str:
    return str(len(value)) + " " + str(value.upper())


def total(values: list[int] | set[int]) -> int:
    return len(values) + sum(values)


print(describe("abc"), describe(b"xy"), total([1, 2]), total({3}))
```

The results join into one type: `value.upper()` above is a
`str | bytes`, and `value[0]` a `str | int`. A union computed in place,
such as `value * 2`, is used member by member once it is assigned to a
variable. Lists, sets and dicts of a union compare, hash and write each
item as the member it holds, so `1` and `1.0` are one key, as in CPython.
One value assigned to several targets, one of which holds a union, as in
`a = items[0] = 1`, is planned for 0.5 (TYP-16). A truth test
whose member classes test themselves with `__len__`, or differently from
their base, is planned for 0.5.

## Names that keep their value

`Final` from `typing` declares a name that its declaration alone assigns.
`Final[T]` gives it the type `T`, and a bare `Final` gives it the type of
its value. A module or a function declares a `Final` variable, a class
body a `Final` class variable, and `__init__` a `Final` attribute with
`self.name: Final = value`:

<!-- typy-expect: accepted -->
```python
from typing import Final

LIMIT: Final = 10


class Box:
    SIZE: Final[int] = 3

    def __init__(self, width: int) -> None:
        self.width: Final = width


box = Box(LIMIT)
print(Box.SIZE, box.SIZE, box.width)
```

Any other assignment is E0414, also through `global`, an augmented
assignment, a `for` target or `module.NAME = value`. A `Final` without a
value, `Final` in any other annotation, such as a parameter's, and a
`Final` attribute declared anywhere but the body of `__init__` are
E0408. CPython runs such a program without a word, since only a type
checker reads `Final`; TyPy refuses it when it compiles the program:

<!-- typy-expect: rejected E0414 at 6:5 -->
```python
from typing import Final

LIMIT: Final = 10

for step in range(2):
    LIMIT += step
```

## Type aliases

`type Name = annotation` at module level makes `Name` stand for the
annotation in every annotation of the module. As in CPython, the value is
read only when an annotation uses the alias, so it may name a class
defined further down, or another alias:

<!-- typy-expect: accepted -->
```python
type Score = int
type Scores = list[Score]
type Found = Node | None


class Node:
    def __init__(self, value: Score) -> None:
        self.value = value


def best(scores: Scores) -> Score:
    return max(scores)


def first(nodes: list[Node]) -> Found:
    return nodes[0] if nodes else None


print(best([3, 9]), first([]))
```

An alias that names itself, such as `type Tree = list[Tree]`, is E0201;
a recursive type is planned for after 1.0. An alias defined twice, E0414,
and one whose name the module also gives a class, a function or a
variable, E0202, are refused, since an annotation would read one meaning
of the name. Type parameters, `type Pair[T] = tuple[T, T]`, come with
generics later in 0.4 (GEN-01). An alias of another module, an alias read
as a value, which CPython gives as a `TypeAliasType`, and `type` inside a
function, a class or a block are planned for 0.5 (SMT-12).

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
