# typy-test: GEN-04, GEN-03
# typy-expect: accepted
from typing import Protocol


class Shape(Protocol):
    sides: int

    def area(self) -> float: ...

    def scaled(self, factor: float) -> float: ...


class Square:
    def __init__(self, side: float) -> None:
        self.side = side
        self.sides = 4

    def area(self) -> float:
        return self.side * self.side

    def scaled(self, factor: float) -> float:
        return self.area() * factor


class Triangle:
    def __init__(self) -> None:
        self.sides = 3

    def area(self) -> float:
        return 0.5

    def scaled(self, factor: float, extra: float = 1.0) -> float:
        return factor * extra


def describe(shape: Shape) -> str:
    return f"{shape.sides} sides, area {shape.area()}"


def total(shapes: list[Shape]) -> float:
    result = 0.0
    for shape in shapes:
        result += shape.scaled(2.0)
    return result


class Report:
    def line(self, shape: Shape, label: str) -> str:
        return label + ": " + describe(shape)
mixed: list[Square | Triangle] = [Square(1.0), Triangle()]
print(total(mixed))
for each in mixed:
    print(describe(each))
