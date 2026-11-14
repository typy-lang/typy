# typy-test: GEN-04
# typy-expect: rejected E0401 at 15:9
from typing import Protocol


class Shape(Protocol):
    def area(self) -> float: ...


class Circle:
    def perimeter(self) -> float:
        return 6.28


shape = Shape()
