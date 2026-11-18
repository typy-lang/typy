# typy-test: FUN-05
# typy-expect: accepted
from typing import Callable


class Shape:
    def __init__(self, name: str) -> None:
        self.name = name


class Circle(Shape):
    def __init__(self, radius: float) -> None:
        super().__init__("circle")
        self.radius = radius


def describe(shape: Shape) -> str:
    return "a " + shape.name


def make_circle() -> Circle:
    return Circle(1.5)


def parse(text: str) -> int:
    return int(text)


def find(words: list[str], word: str) -> int | None:
    return words.index(word) if word in words else None


def pair(value: float) -> tuple[float, bool]:
    return (value / 2, value > 1)


def report(flag: bool) -> None:
    print("report", flag)


# A function of any shape fits a callable of circles, and one that makes a circle makes
# a shape, so no value is converted through the call.
on_circle: Callable[[Circle], str] = describe
maker: Callable[[], Shape] = make_circle
print(on_circle(Circle(2.0)), maker().name)
reader = parse
try:
    reader("seven")
except ValueError as error:
    print("ValueError:", error)
lookup = find
print(lookup(["a", "b"], "b"), lookup([], "z"))
halve = pair
print(halve(3.0), halve(0.5)[1])
announce: Callable[[bool], None] = report
announce(True)
