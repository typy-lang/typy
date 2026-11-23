# typy-test: CLS-45
# typy-expect: rejected E0434 at 13:24
from abc import ABC, abstractmethod


class Shape(ABC):
    @abstractmethod
    def area(self) -> float: ...


class Square(Shape):
    def area(self) -> float:
        return super().area() + 1.0


print(Square().area())
