# typy-test: GEN-17
# typy-expect: rejected E0201 at 10:14
from typing import Protocol


class Shape(Protocol):
    def area(self) -> float: ...


class Square(Shape):
    def area(self) -> float:
        return 1.0
