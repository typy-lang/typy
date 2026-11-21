# typy-test: CLS-18
# typy-expect: accepted
from typing import Optional, Union


class Shape:
    def __init__(self, size: int) -> None:
        self.size = size


class Square(Shape):
    pass


def check(kind: type[Shape], value: Shape | int | None) -> str:
    if isinstance(value, Union[kind, int]):
        return "kind or number"
    if isinstance(value, Optional[kind]):
        return "nothing"
    return "other"


print(check(Square, Square(1)), check(Square, 3), check(Square, None), check(Square, Shape(2)))
