# typy-test: CLS-06
# typy-expect: accepted
class Shape:
    count = 0

    def __init__(self, name: str) -> None:
        self.name = name

    @classmethod
    def unit(cls) -> "Shape":
        cls.count += 1
        return cls("unit " + cls.__name__)

    @classmethod
    def many(cls, n: int) -> "list[Shape]":
        return [cls.unit() for _ in range(n)]

    def describe(self) -> str:
        return self.name


class Square(Shape):
    count = 100

    def describe(self) -> str:
        return "square " + self.name


class Circle(Shape):
    @classmethod
    def unit(cls) -> "Shape":
        return cls("round")


for shape in [Shape.unit(), Square.unit(), Circle.unit()] + Square.many(2):
    print(type(shape).__name__, shape.describe())
print(Shape.count, Square.count, Circle.count)
