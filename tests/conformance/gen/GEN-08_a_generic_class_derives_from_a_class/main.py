# typy-test: GEN-08, GEN-14
# typy-expect: accepted
from dataclasses import dataclass

import shapes


class Base:
    def __init__(self, name: str) -> None:
        self.name = name

    def describe(self) -> str:
        return "base " + self.name + " " + self.extra()

    def extra(self) -> str:
        return "-"

    def __eq__(self, other: "Base") -> bool:
        return other.name == self.name


class Box[T](Base):
    def __init__(self, item: T) -> None:
        super().__init__("box")
        self.item = item

    def get(self) -> T:
        return self.item

    def extra(self) -> str:
        return str(self.item)


def show(b: Base) -> str:
    return b.describe()


b = Box(1)
s = Box("s")
print(b.get() + 1, s.get().upper(), b.name, s.describe())
print(show(b), show(s), [show(each) for each in [Base("plain"), b]])
print(isinstance(b, Base), isinstance(b, Box), type(s) is Box, b == s, b == Base("box"))
items: list[Base] = [b, s, Base("x")]
print([isinstance(each, Box) for each in items])


def check(x: Base) -> str:
    if isinstance(x, Box):
        return "box " + x.name
    return "base " + x.name


print(check(Box(1)), check(Base("b")))


class Scaled[N: (int, float)](shapes.Shape):
    def __init__(self, size: N) -> None:
        super().__init__("scaled")
        self.size = size

    def area(self) -> float:
        return float(self.size * self.size)

    def doubled(self) -> N:
        return self.size + self.size


@dataclass
class Point:
    x: int


@dataclass(eq=False)
class Tagged[T](Point):
    tag: T


small = Scaled(3)
large = Scaled(1.5)
print(small.report(), large.report(), small.doubled(), large.doubled())
t = Tagged(1, "a")
print(t, t.x, t.tag, t == t, t == Point(1), isinstance(t, Point))
all_shapes: list[shapes.Shape] = [small, large, shapes.Shape("plain")]
print([each.report() for each in all_shapes])
