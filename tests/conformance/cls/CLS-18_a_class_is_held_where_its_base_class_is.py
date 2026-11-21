# typy-test: CLS-18
# typy-expect: accepted
class Shape:
    sides = 0

    def __init__(self, size: int) -> None:
        self.size = size

    @classmethod
    def unit(cls) -> "Shape":
        return cls(1)


class Square(Shape):
    sides = 4


def check(kind: type[Shape], value: Shape) -> str:
    if isinstance(value, kind):
        return kind.__name__
    return "other"


kinds: tuple[type[Shape], ...] = (Square, Shape)
print(check(Square, Square(1)), check(Square, Shape(2)), check(Shape, Square(3)))
print(isinstance(Square(1), kinds), issubclass(Square, kinds), Shape.unit().size, type(Square.unit()).__name__)
chosen: type[Shape] = Square
print(chosen, chosen is Square, chosen == Shape)
