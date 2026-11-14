# typy-test: GEN-05
# typy-expect: rejected E0401 at 15:28
from typing import Protocol


class Shape(Protocol):
    def area(self) -> float: ...


class Circle:
    def perimeter(self) -> float:
        return 6.28


print(isinstance(Circle(), Shape))
