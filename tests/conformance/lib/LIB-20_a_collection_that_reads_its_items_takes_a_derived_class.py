# typy-test: LIB-20
# typy-expect: accepted
from collections.abc import Collection, Mapping, MutableSequence, Sequence
from typing import AbstractSet


class Shape:
    def __init__(self, size: int) -> None:
        self.size = size


class Square(Shape):
    pass


def total(shapes: Sequence[Shape]) -> int:
    return sum(shape.size for shape in shapes)


def largest(table: Mapping[str, Shape]) -> int:
    return max(shape.size for shape in table.values())


def count(items: Collection[int]) -> int:
    return len(items)


def keys(items: AbstractSet[str]) -> list[str]:
    return sorted(items)


squares = [Square(1), Square(2)]
print(total(squares), total((Square(3), Shape(4))), largest({"a": Square(5)}))
print(count({1: "a"}.keys()), count({"a": 1, "b": 2}.values()), keys({"x": 1, "y": 2}.keys()))
grow: MutableSequence[int] = [1]
grow.append(2)
print(grow, count(range(3)), count(b"ab"))
