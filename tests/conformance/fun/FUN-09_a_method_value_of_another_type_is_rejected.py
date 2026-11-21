# typy-test: FUN-09
# typy-expect: rejected E0402 at 18:31
from collections.abc import Callable


class Shape:
    def sides(self) -> int:
        return 0


class Square(Shape):
    @staticmethod
    def sides() -> int:
        return 4


shape: Shape = Square()
count: Callable[[int], int] = shape.sides
