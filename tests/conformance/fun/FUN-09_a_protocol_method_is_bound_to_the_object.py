# typy-test: FUN-09
# typy-expect: accepted
from collections.abc import Callable
from typing import Protocol


class Shape(Protocol):
    def area(self, scale: int) -> float: ...


class Square:
    def __init__(self, side: float) -> None:
        self.side = side

    def area(self, scale: int) -> float:
        return self.side * self.side * scale


class Disc:
    def __init__(self, radius: float) -> None:
        self.radius = radius

    def area(self, scale: int | None) -> float:
        return 3.0 * self.radius * self.radius * (1 if scale is None else scale)


def measure(shape: Shape) -> float:
    method = shape.area
    return method(2)


square = Square(2.0)
disc = Disc(1.0)
print(measure(square), measure(disc))
held: Shape = square
read: Callable[[int], float] = held.area
print(read(3), held.area == square.area, held.area == held.area, repr(held.area)[:30])
held = disc
print(held.area == disc.area, held.area != Disc(1.0).area, repr(held.area)[:28])
