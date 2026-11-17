# typy-test: GEN-10, GEN-07, GEN-03
# typy-expect: accepted
from typing import Protocol

from shapes import Big, Box, Circle


class Shape(Protocol):
    def area(self) -> float: ...


class Holder:
    def __init__(self, shape: Shape) -> None:
        self.shape = shape

    def doubled(self) -> float:
        return self.shape.area() * 2


def make(kind: int) -> Shape:
    if kind == 0:
        return Circle(1.0)
    if kind == 1:
        return Big(2.0)
    return Box("x")


def maybe(flag: bool) -> Shape | None:
    return Circle(0.5) if flag else None


shapes: list[Shape] = [make(0), make(1), make(2), Box(3)]
print([shape.area() for shape in shapes])
print(Holder(Big(1.0)).doubled(), Holder(make(0)).shape.area())
found = maybe(True)
if found is not None:
    print(found.area())
print(maybe(False) is None)
first = shapes[0]
if isinstance(first, Circle):
    print("a circle of", first.radius)
print(first)
named: dict[str, Shape] = {"one": Circle(2.0)}
print(named["one"].area(), len(named))
