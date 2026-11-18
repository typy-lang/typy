# typy-test: MEM-03
# typy-expect: accepted
from typing import Callable


class Point:
    def __init__(self, x: int) -> None:
        self.x = x


kept: list[Point] = []


def keep(point: Point) -> None:
    kept.append(point)


def make(store: Callable[[Point], None], x: int) -> None:
    # The code a value runs is known only when the program runs, so the point is never
    # held in this function's frame.
    point = Point(x)
    store(point)


for x in range(3):
    make(keep, x)
print([point.x for point in kept])
