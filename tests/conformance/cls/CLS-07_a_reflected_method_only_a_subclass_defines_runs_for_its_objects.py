# typy-test: CLS-07
# typy-expect: accepted
class Shape:
    def __init__(self, size: int) -> None:
        self.size = size

    def __add__(self, other: "Shape") -> int:
        print("Shape.__add__")
        return self.size + other.size


class Square(Shape):
    def __radd__(self, other: "Shape | int") -> int:
        print("Square.__radd__")
        if isinstance(other, Shape):
            return other.size * 100 + self.size
        return other * 10 + self.size


class Disc(Shape):
    pass


def add(left: Shape, right: Shape) -> int:
    return left + right


def add_number(right: Shape) -> int:
    return 5 + right


print(add(Shape(1), Shape(2)), add(Shape(1), Square(2)), add(Square(1), Square(2)))
print(add(Disc(1), Square(2)), add(Square(1), Disc(2)))
print(add_number(Square(3)))
try:
    print(add_number(Disc(3)))
except TypeError as error:
    print("TypeError:", error)
