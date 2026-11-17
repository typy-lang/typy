# typy-test: CLS-25
# typy-expect: accepted
class Shape:
    sides = 0
    made = 0

    def describe(self) -> str:
        return type(self).__name__ + " " + str(self.sides)

    @classmethod
    def count(cls) -> int:
        return cls.sides


class Square(Shape):
    pass


class Tile(Square):
    pass


class Circle(Shape):
    pass


shapes: list[Shape] = [Shape(), Square(), Tile(), Circle()]
print([shape.describe() for shape in shapes], Square.sides, Tile.count())
Square.sides = 4
print([shape.describe() for shape in shapes], Shape.sides, Square.sides, Tile.sides, Tile.count())
Shape.sides = 1
Square.sides += 1
print([shape.sides for shape in shapes], Circle.count(), Square.count())


def reset() -> None:
    Tile.sides = 9


reset()
print(Tile.sides, Square.sides, Shape.made, [shape.describe() for shape in shapes])
