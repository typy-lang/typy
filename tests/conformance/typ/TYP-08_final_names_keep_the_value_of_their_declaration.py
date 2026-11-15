# typy-test: TYP-08
# typy-expect: accepted
from typing import Final
import typing

LIMIT: Final = 10
NAME: Final[str] = "box"
RATE: typing.Final[float] = 0.5


class Box:
    SIZE: Final = 3
    KIND: Final[str] = "plain"

    def __init__(self, width: int) -> None:
        self.width: Final = width
        self.label: Final[str] = NAME + str(width)

    def area(self) -> int:
        return self.width * Box.SIZE


def main() -> None:
    total: Final = LIMIT + 1
    box = Box(4)
    print(LIMIT, NAME, RATE, total)
    print(Box.SIZE, Box.KIND, box.SIZE, box.width, box.label, box.area())


main()
