# typy-test: CLS-27
# typy-expect: accepted
from typing import NamedTuple


class Point(NamedTuple):
    x: int
    y: int


def show(value: Point | tuple[int, int] | int) -> None:
    print(isinstance(value, tuple), isinstance(value, Point), value)


points = [Point(1, 2), Point(3, 4)]
print(points == [Point(1, 2), Point(3, 4)], points.index(Point(3, 4)), Point(3, 4) in points)
counts = {Point(1, 2): "a"}
print(counts[Point(1, 2)], Point(1, 2) in {Point(1, 2)})
maybe: Point | None = Point(5, 6)
if maybe is not None:
    print(maybe.x, maybe[1])
show(Point(7, 8))
show((7, 8))
show(9)
print(max(points), min(points), sorted(points, reverse=True))
a, b = points
print(a, b)
print(Point(1, 2) != Point(1, 3), Point(1, 2) <= Point(1, 2), Point(2, 0) > Point(1, 9))
