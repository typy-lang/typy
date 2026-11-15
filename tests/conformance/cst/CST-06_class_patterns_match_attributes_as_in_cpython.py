# typy-test: CST-06
# typy-expect: accepted
from dataclasses import dataclass


class Shape:
    def __init__(self, name: str) -> None:
        self.name = name


class Circle(Shape):
    __match_args__ = ("radius",)

    def __init__(self, radius: int) -> None:
        super().__init__("circle")
        self.radius = radius


class Square(Shape):
    __match_args__ = ("side", "name")

    def __init__(self, side: int) -> None:
        super().__init__("square")
        self.side = side


@dataclass
class Point:
    x: int
    y: int


@dataclass
class Line:
    start: Point
    end: Point


def area(shape: Shape) -> int:
    match shape:
        case Circle(0):
            return 0
        case Circle(radius=r):
            return 3 * r * r
        case Square(side, "square"):
            return side * side
        case Shape(name=other):
            print("unknown", other)
            return -1


def where(point: Point | None) -> str:
    match point:
        case None:
            return "nowhere"
        case Point(0, 0):
            return "origin"
        case Point(x, 0) | Point(0, x):
            return f"axis {x}"
        case Point(x=x, y=y) if x == y:
            return f"diagonal {x}"
        case Point(x, y):
            return f"at {x},{y}"


def length(line: Line) -> int:
    match line:
        case Line(Point(x1, y1), Point(x2, y2)):
            return abs(x2 - x1) + abs(y2 - y1)


def kind(value: int | str | float | list[int]) -> str:
    match value:
        case int(0):
            return "zero"
        case int(n):
            return f"int {n}"
        case str() as text:
            return f"str {text}"
        case float(f) if f > 1.0:
            return f"big float {f}"
        case list([first, *_]):
            return f"list from {first}"
        case _:
            return "other"


def failure(error: Exception) -> str:
    match error:
        case KeyError():
            return "key"
        case ValueError() | TypeError():
            return "value or type"
        case _:
            return str(error)


print(area(Circle(0)), area(Circle(2)), area(Square(3)), area(Shape("blob")))
for point in [None, Point(0, 0), Point(5, 0), Point(0, 6), Point(4, 4), Point(1, 2)]:
    print(where(point))
print(length(Line(Point(1, 1), Point(4, 5))))
values: list[int | str | float | list[int]] = [0, 7, "a", 2.5, 0.5, [9, 8], []]
for value in values:
    print(kind(value))
print(failure(KeyError("k")), failure(TypeError("t")), failure(RuntimeError("r")))
print(Point.__match_args__)
