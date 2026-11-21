# typy-test: LIB-20
# typy-expect: rejected E0404 at 19:6
from collections.abc import MutableSequence


class Shape:
    pass


class Square(Shape):
    pass


def grow(shapes: MutableSequence[Shape]) -> None:
    shapes.append(Shape())


squares = [Square()]
grow(squares)
