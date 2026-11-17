# typy-test: CLS-15, CLS-16, CLS-17
# typy-expect: accepted
from dataclasses import dataclass, field

calls: list[int] = []


def make_tags() -> list[str]:
    calls.append(1)
    return ["new"]


def origin() -> "Point":
    return Point(0, 0)


@dataclass(frozen=True)
class Point:
    x: int
    y: int = 0


@dataclass
class Bag:
    name: str
    items: list[int] = field(default_factory=list)
    tags: list[str] = field(default_factory=make_tags)
    start: Point = field(default_factory=origin)


first = Bag("a")
second = Bag("b", [1])
first.items.append(5)
print(first, second, len(calls))
print(first.items == second.items, Bag("c").items)
print({Point(1, 2), Point(1, 2)}, {Point(3): "p"}[Point(3, 0)], Point(3) == Point(3, 0))


@dataclass(unsafe_hash=True)
class Key:
    name: str
    size: int = 1


print(len({Key("k"), Key("k", 1)}), {Key("k"): 1}[Key("k", 1)])


@dataclass(init=False, repr=False)
class Manual:
    value: int = 3

    def __init__(self) -> None:
        self.value = 4


manual = Manual()
print(manual.value, repr(manual)[:8])


@dataclass(slots=True)
class Slim:
    a: int
    b: str = "b"


slim = Slim(1)
print(slim, slim.b)


@dataclass(eq=False)
class Plain:
    n: int


p = Plain(1)
print(len({p, p}), Plain(1) == Plain(1))
