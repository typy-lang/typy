# typy-test: GEN-10
# typy-expect: rejected E0201 at 15:1
from typing import Protocol


class Shape(Protocol):
    def area(self) -> float: ...


class Circle:
    def perimeter(self) -> float:
        return 6.28


def make() -> Shape:
    return Circle()
