# typy-test: GEN-10, GEN-04
# typy-expect: accepted
from typing import Protocol


class Shape(Protocol):
    name: str

    def area(self) -> float: ...

    def scaled(self, factor: float) -> float: ...


class Circle:
    def __init__(self, radius: float) -> None:
        self.name = "circle"
        self.radius = radius

    def area(self) -> float:
        return 3.0 * self.radius * self.radius

    def scaled(self, factor: float) -> float:
        return self.area() * factor


class Square:
    def __init__(self, side: int) -> None:
        self.name = "square"
        self.side = side

    def area(self) -> float:
        return float(self.side * self.side)

    def scaled(self, factor: float, extra: float = 0.0) -> float:
        return self.area() * factor + extra


def largest(shapes: list[Shape]) -> Shape:
    best = shapes[0]
    for shape in shapes:
        if shape.area() > best.area():
            best = shape
    return best


shapes: list[Shape] = [Circle(1.0), Square(2), Circle(0.5)]
best = largest(shapes)
print(best.name, best.area(), best.scaled(2.0))
print([shape.name for shape in shapes], sum([shape.area() for shape in shapes], 0.0))
one: Shape = Square(3)
print(one.name, one.scaled(1.0))
