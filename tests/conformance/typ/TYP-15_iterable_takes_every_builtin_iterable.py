# typy-test: TYP-15, CST-09
# typy-expect: accepted
from collections.abc import Iterable, Iterator
import typing
from types import TracebackType, ModuleType
import collections.abc


def total(values: Iterable[int]) -> int:
    result = 0
    for value in values:
        result += value
    return result


def twice(values: Iterable[int]) -> list[int]:
    first = [value for value in values]
    second = list(values)
    return first + second


def names(items: typing.Iterable[tuple[str, int]]) -> list[str]:
    return [name for name, _ in items]


def first(values: collections.abc.Iterator[str]) -> str:
    return next(values, "none")


def words(text: Iterable[str]) -> str:
    return "-".join(sorted(text))


def counter(limit: int) -> Iterator[int]:
    return iter(range(limit))


ages = {"ada": 36, "alan": 41}
print(total([1, 2, 3]), total({4, 5}), total(range(5)), total(ages.values()), total(b"ab"))
print(total((7, 8, 9)), total(iter([10, 11])), total(counter(4)), sum([1]) + total([]))
print(twice([1, 2]), twice(range(3)), twice(iter([5, 6])))
print(names(ages.items()), names([("x", 1)]), words("cab"), words(ages), words(ages.keys()))
empty: list[str] = []
print(first(iter(["a", "b"])), first(iter(empty)))
values: Iterable[int] = [1, 2]
print(total(values), sorted(values), max(values), min(values))
seen: Iterable[str] = ages
print(list(seen), [w for w in iter(seen)])


def show(module: ModuleType) -> None:
    print(module)


def describe(trace: TracebackType | None) -> bool:
    return trace is None


print(describe(None))
