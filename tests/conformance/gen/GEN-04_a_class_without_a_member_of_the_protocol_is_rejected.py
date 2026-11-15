# typy-test: GEN-04
# typy-expect: rejected E0404 at 19:7
from typing import Protocol


class Shape(Protocol):
    def area(self) -> float: ...


class Circle:
    def perimeter(self) -> float:
        return 6.28


def describe(shape: Shape) -> float:
    return shape.area()


print(describe(Circle()))
