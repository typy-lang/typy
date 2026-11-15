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
condition on it are refused.

The older names of `typing` spell the types above: `Optional[int]` is
`int | None`, `Union[int, str]` is `int | str`, and `List`, `Dict`, `Set`,
`Tuple` and `Type` are `list`, `dict`, `set`, `tuple` and `type` (LIB-17).
`Iterable` and `Iterator`, of either module, and `List`, `Dict`, `Set`,
`Tuple`, `Type`, `Optional[...]` and `Union[...]` of `typing` may also be
the class of `isinstance` and of a class pattern, answered as CPython
answers them: a list is `Iterable` and not an `Iterator`, its iterator is
both, and a class is `Iterable` when it, or a base, defines `__iter__`
(TYP-17).

<!-- typy-expect: accepted -->
```python
from typing import Optional, Union, List
from collections.abc import Iterable, Iterator


class Bag:
    def __iter__(self) -> Iterator[int]:
        return iter([1, 2])


def describe(value: Optional[Union[int, List[int], Bag]]) -> str:
    if isinstance(value, Optional[int]):
        return "a number or nothing"
    if isinstance(value, Iterable):
        return "something to loop over"
    return "other"


print(describe(None), describe(3), describe([1]), describe(Bag()))
```

As in CPython, a bare `Optional` and a subscripted name such as
`List[int]` in `isinstance` are E0401, and a bare `Union` is E0202, since
CPython 3.12 and 3.13 raise `TypeError` where 3.14 and 3.15 answer
`False`. The names held in a variable, passed or printed are planned for
1.0 (TYP-18), and so are `Sequence`, `Mapping` and the other abstract
collections (LIB-20), `NamedTuple` (CLS-27) and `Callable`, with function
values (FUN-05). `typing.Any` and `typing.cast` are refused, since every
value in TyPy has one type (TYP-10).

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
One value assigned to several targets, as in `a = items[0] = 1`, or
unpacked into targets, is held by each target as its own type, a union
or an optional among them (TYP-16). A truth test
whose member classes test themselves with `__len__`, or differently from
their base, is planned for 0.5 (CLS-24).

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
of the name.

An alias may take type parameters. `Pair[int]` stands for the value with
`int` in place of `T`, and a type parameter of a generic function or
class may be the argument:

<!-- typy-expect: accepted -->
```python
type Pair[T] = tuple[T, T]
type Table[K, V] = dict[K, list[V]]


def swap[T](pair: Pair[T]) -> Pair[T]:
    return (pair[1], pair[0])


counts: Table[str, int] = {"a": [1, 2]}
print(swap((1, 2)), swap(("x", "y")), counts)
```

A generic alias named without one type argument for each parameter, such
as `Pair` or `Pair[int, int]`, is E0408. An alias of another module is
read as its own module wrote it, imported by name, as in
`from shapes import Pair`, or read through its module, as `shapes.Pair[int]`.
A `type` statement inside a function, a class or a block makes its alias
for the statements after it there. A bound or constraints on an alias's
type parameter, as in `type Ordered[T: float] = tuple[T, T]`, are taken as
CPython takes them, which checks no argument against them; each use is
checked as the type the alias stands for (SMT-12).

Read as a value, an alias of the module's top level, or one imported, is
a `typing.TypeAliasType`, as in CPython: it is written as its name, has
`__name__` and `__module__`, and is the same object wherever it is read.

<!-- typy-expect: accepted -->
```python
type Score = int
kinds = [Score]
print(Score, Score.__name__, Score.__module__, type(Score), kinds[0] is Score)
```

As in CPython, calling an alias, or giving it to `isinstance`, is E0401.
An alias a function or a class defines read as a value, whose text
CPython 3.14 changed, an alias given its type arguments as a value, and
an alias's `__value__` and `__type_params__` are planned for 1.0
(TYP-18). Two aliases compare with `is`; `==` between them is E0407, as
for module objects.

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
