# typy-test: EXP-22
# typy-expect: accepted
from typing import Protocol


class Shape(Protocol):
    def scaled(self, by: int, extra: int = 0) -> int: ...


class Square:
    def __init__(self, side: int) -> None:
        self.side = side

    def scaled(self, by: int, extra: int = 0) -> int:
        return self.side * by + extra


class Line:
    def scaled(self, by: int, extra: int = 0) -> int:
        return by - extra


def run(shape: Shape, values: list[int], options: dict[str, int]) -> str:
    try:
        return str(shape.scaled(*values, **options))
    except TypeError as error:
        return str(error)


square = Square(3)
print(run(square, [2], {}), run(square, [2, 1], {}), run(square, [2], {"extra": 5}))
print(run(square, [2, 1, 9], {}))
print(run(square, [], {}))
print(run(square, [2], {"by": 1}))
print(run(Line(), [], {"by": 7, "extra": 2}), run(Line(), [7], {}))
print(run(Line(), [], {"extra": 2}))
