# typy-test: CLS-27
# typy-expect: accepted
from typing import NamedTuple


class Point(NamedTuple):
    x: int
    y: int = 0

    @property
    def doubled(self) -> int:
        return 2 * self.x

    @classmethod
    def make(cls, v: int) -> "Point":
        return cls(v, v)

    @staticmethod
    def unit() -> int:
        return 1


class Labelled(Point):
    def label(self) -> str:
        return "P" + str(self.x)


p = Point(3)
print(p.doubled, Point.make(2), Point.unit(), p.__match_args__)
match p:
    case (a, b):
        print("seq", a, b)
match p:
    case Point(x=3, y=yy):
        print("cls", yy)
match p:
    case Point(a, b):
        print("pos", a, b)
lab = Labelled(4, 5)
print(lab, lab.label(), lab.x, isinstance(lab, Point), lab == (4, 5))
first, second = lab
print(first, second, len(lab))


class Pair[T](NamedTuple):
    first: T
    second: T


print(Pair(1, 2), Pair("a", "b").first, Pair(1, 2) == (1, 2))
match lab:
    case (one, *rest):
        print(one, rest)
