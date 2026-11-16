# typy-test: GEN-10, CLS-07
# typy-expect: accepted
from typing import Protocol


class Shape(Protocol):
    def area(self) -> float: ...


class Circle:
    def __init__(self, radius: float) -> None:
        self.radius = radius

    def area(self) -> float:
        return self.radius

    def __eq__(self, other: "Circle") -> bool:
        return self.radius == other.radius

    def __hash__(self) -> int:
        return 7


a: Shape = Circle(1.0)
b: Shape = Circle(1.0)
print(a == b, a != b, a is b, len({a, b}), a in [b])
