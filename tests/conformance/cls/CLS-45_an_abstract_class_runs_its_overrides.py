# typy-test: CLS-45
# typy-expect: accepted
# Abstract methods run through their overrides; a class method that calls `cls()` on a class
# with abstract methods left raises CPython's TypeError, and an empty abstract method whose
# result holds None returns it through super().
from abc import ABC, abstractmethod


class Shape(ABC):
    def __init__(self, label: str) -> None:
        self.label = label

    @abstractmethod
    def area(self) -> float:
        """Returns the area."""

    @property
    @abstractmethod
    def sides(self) -> int: ...

    @abstractmethod
    def note(self) -> str | None: ...

    def describe(self) -> str:
        return f"{self.label}: {self.area():.2f} with {self.sides} sides"

    @classmethod
    def make(cls, label: str) -> "Shape":
        return cls(label)


class Square(Shape):
    def area(self) -> float:
        return 4.0

    @property
    def sides(self) -> int:
        return 4

    def note(self) -> str | None:
        return super().note()


class Circle(Shape):
    def area(self) -> float:
        return 3.14

    @property
    def sides(self) -> int:
        return 0

    def note(self) -> str | None:
        return "round"


shapes: list[Shape] = [Square("square"), Circle("circle"), Circle.make("made")]
for shape in shapes:
    print(shape.describe(), shape.note())
try:
    Shape.make("abstract")
except TypeError as error:
    print(error)
