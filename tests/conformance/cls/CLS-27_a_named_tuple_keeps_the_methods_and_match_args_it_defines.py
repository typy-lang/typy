# typy-test: CLS-27
# typy-expect: accepted
from typing import NamedTuple


class Point(NamedTuple):
    x: int
    y: int

    def __repr__(self) -> str:
        return f"P<{self.x}>"

    def __eq__(self, other: object) -> bool:
        return isinstance(other, Point) and other.x == self.x

    def __hash__(self) -> int:
        return self.x


class Pair(NamedTuple):
    a: int
    b: int
    __match_args__ = ("b",)


p = Point(1, 2)
print(p, p == Point(1, 5), {p: 1}[Point(1, 9)])
match Pair(3, 4):
    case Pair(b):
        print(b)
