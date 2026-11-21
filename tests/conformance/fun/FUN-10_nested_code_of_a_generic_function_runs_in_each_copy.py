# typy-test: FUN-10
# typy-expect: accepted
from collections.abc import Callable, Iterator
from typing import Protocol


def pair_with[T](first: T) -> Callable[[T], tuple[T, T]]:
    def make(second: T) -> tuple[T, T]:
        return (first, second)

    return make


def scaled[T: (int, float)](values: list[T], factor: T) -> list[T]:
    times: Callable[[T], T] = lambda value: value * factor
    return [times(value) for value in values]


def doubled[T](values: list[T]) -> Iterator[tuple[T, T]]:
    return ((value, value) for value in values)


def counter[T](start: T) -> Callable[[], list[T]]:
    seen: list[T] = []

    def add() -> list[T]:
        nonlocal seen
        seen = seen + [start]
        return seen

    return add


make = pair_with(1)
print(make(2), pair_with("a")("b"), scaled([1, 2], 3), scaled([1.5], 2.0))
print(list(doubled(["x", "y"])), counter(7)(), repr(make).startswith("<function pair_with.<locals>.make at 0x"))


class Shape(Protocol):
    def area(self) -> float: ...


class Square:
    def __init__(self, side: float) -> None:
        self.side = side

    def area(self) -> float:
        return self.side * self.side


def total(shape: Shape, count: int) -> float:
    def times(value: float) -> float:
        return value * count

    return times(shape.area())


print(total(Square(2.0), 3))
