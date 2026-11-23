# typy-test: TYP-30
# typy-expect: accepted
from typing import Self


class Shape:
    def __init__(self, size: float) -> None:
        self.size = size

    def grown(self, factor: float) -> Self:
        self.size = self.size * factor
        return self

    def same(self, other: Self) -> bool:
        return self.size == other.size

    def pick(self, other: Self, larger: bool) -> Self:
        if larger and other.size > self.size:
            return other
        return self

    def maybe(self, keep: bool) -> Self | None:
        if keep:
            return self
        return None

    @classmethod
    def unit(cls) -> Self:
        return cls(1.0)


class Circle(Shape):
    def grown(self, factor: float) -> Self:
        super().grown(factor)
        return self

    def label(self) -> str:
        return "circle " + str(self.size)


c = Circle(1.0).grown(2.0).grown(1.5)
print(c.label(), Shape.same(c, Shape(3.0)), c.same(Circle(3.0)))
bigger: Circle = c.pick(Circle(9.0), True)
kept = c.maybe(True)
if kept is not None:
    print(kept.label())
print(c.maybe(False), bigger.label(), c.unit().label(), Circle.unit().label())
