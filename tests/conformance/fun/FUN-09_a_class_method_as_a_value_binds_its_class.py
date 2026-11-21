# typy-test: FUN-09
# typy-expect: accepted
from collections.abc import Callable


class Box:
    def __init__(self, size: int) -> None:
        self.size = size

    @classmethod
    def make(cls, size: int) -> "Box":
        return cls(size)

    def kind(self) -> str:
        return "box"


class BigBox(Box):
    def kind(self) -> str:
        return "big"


make: Callable[[int], Box] = Box.make
big_make: Callable[[int], Box] = BigBox.make
box: Box = BigBox(3)
through: Callable[[int], Box] = box.make
print(make(1).kind(), big_make(2).kind(), through(4).kind(), through(4).size)
print(make == Box.make, make is Box.make, make == big_make, through == big_make)
print(make)
