# typy-test: LIB-20
# typy-expect: accepted
from collections.abc import Collection, Mapping, MutableSequence, Reversible, Sequence
from typing import AbstractSet


def total(values: Sequence[int]) -> int:
    result = 0
    for value in values:
        result += value
    return result


def first(values: Sequence[str]) -> str:
    return values[0] + values[-1] + str(len(values))


def describe(table: Mapping[str, int]) -> str:
    return ",".join(sorted(table)) + str(table["a"]) + str(len(table))


def grow(items: MutableSequence[int]) -> None:
    items.append(9)


def size(items: Collection[int]) -> int:
    return len(items) + (1 if 3 in items else 0)


def keys(items: AbstractSet[str]) -> int:
    return len(items)


def backwards(items: Reversible[int]) -> list[int]:
    return sorted(items, reverse=True)


numbers = [1, 2, 3]
grow(numbers)
print(total(numbers), total((4, 5)), total(range(4)), total(b"ab"))
print(first(["x", "y"]), first(("p", "q", "r")), first("hello"))
print(describe({"a": 1, "b": 2}), numbers, size({3, 4}), size([1]), size(range(5)))
print(keys({"k"}), backwards([1, 2, 3]), backwards(range(3)))
