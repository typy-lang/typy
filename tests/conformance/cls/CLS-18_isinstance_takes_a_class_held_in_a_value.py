# typy-test: CLS-18
# typy-expect: accepted
class Shape:
    def name(self) -> str:
        return "shape"


class Square(Shape):
    def side(self) -> int:
        return 4


class Circle(Shape):
    pass


def pick(flag: bool) -> type[Shape]:
    return Shape


shapes: list[Shape] = [Square(), Circle(), Shape()]
kind = type(shapes[0])
square_class: type[Square] = Square
for shape in shapes:
    print(isinstance(shape, kind), isinstance(shape, square_class), isinstance(shape, (Circle, kind)))
    if isinstance(shape, square_class):
        print("side", shape.side())
maybe: Shape | None = None
print(isinstance(maybe, kind), isinstance(None, type(None)), isinstance(True, type(1)), isinstance(3, pick(True)))
