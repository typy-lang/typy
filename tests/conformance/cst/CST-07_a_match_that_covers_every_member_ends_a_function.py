# typy-test: CST-07
# typy-expect: accepted
from dataclasses import dataclass


@dataclass
class Circle:
    radius: int


@dataclass
class Square:
    side: int


@dataclass
class Dot:
    pass


type Shape = Circle | Square | Dot


def area(shape: Shape) -> int:
    match shape:
        case Circle(r):
            return 3 * r * r
        case Square(side=s):
            return s * s
        case Dot():
            return 0


def flag(state: bool) -> str:
    match state:
        case True:
            return "on"
        case False:
            return "off"


def maybe(value: int | None) -> int:
    match value:
        case None:
            return 0
        case int(n):
            return n


def pair(values: tuple[int, str]) -> str:
    match values:
        case (n, text):
            return text * n


def items(values: list[int] | dict[str, int]) -> int:
    match values:
        case [*all_items]:
            return len(all_items)
        case {**table}:
            return len(table)


def partial(shape: Shape) -> str:
    match shape:
        case Circle():
            return "round"
        case Square(side=1):
            return "unit"
    return "other"


def guarded(state: bool | None) -> str:
    match state:
        case True:
            return "yes"
        case None if state is None:
            return "none"
    return "no"


print(area(Circle(1)), area(Square(2)), area(Dot()), flag(True), flag(False))
print(maybe(None), maybe(4), pair((2, "ab")), items([1, 2]), items({"a": 1}))
print(partial(Circle(1)), partial(Square(1)), partial(Dot()), guarded(None), guarded(False))
