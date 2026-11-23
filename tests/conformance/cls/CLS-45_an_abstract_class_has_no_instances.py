# typy-test: CLS-45
# typy-expect: rejected E0404 at 12:13
from abc import ABC, abstractmethod


class Shape(ABC):
    @abstractmethod
    def area(self) -> float: ...


def total() -> float:
    shape = Shape()
    return shape.area()


print(total())
