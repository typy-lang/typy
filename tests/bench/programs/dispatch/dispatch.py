"""Method calls through a base class, as an object-oriented program makes them."""

import sys


class Shape:
    def area(self) -> float:
        return 0.0


class Square(Shape):
    def __init__(self, side: float) -> None:
        self.side = side

    def area(self) -> float:
        return self.side * self.side


class Circle(Shape):
    def __init__(self, radius: float) -> None:
        self.radius = radius

    def area(self) -> float:
        return 3.14159 * self.radius * self.radius


class Rectangle(Shape):
    def __init__(self, width: float, height: float) -> None:
        self.width = width
        self.height = height

    def area(self) -> float:
        return self.width * self.height


def shapes() -> list[Shape]:
    made: list[Shape] = []
    for i in range(1000):
        size = float(i % 7 + 1)
        if i % 3 == 0:
            made.append(Square(size))
        elif i % 3 == 1:
            made.append(Circle(size))
        else:
            made.append(Rectangle(size, size + 1.0))
    return made


def total(made: list[Shape], rounds: int) -> float:
    found = 0.0
    for _ in range(rounds):
        for shape in made:
            found += shape.area()
    return found


def main() -> None:
    print(f"{total(shapes(), int(sys.argv[1])):.3f}")


main()
