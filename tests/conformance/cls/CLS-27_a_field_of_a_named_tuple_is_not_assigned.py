# typy-test: CLS-27
# typy-expect: rejected E0414 at 11:1
from typing import NamedTuple


class Point(NamedTuple):
    x: int


point = Point(1)
point.x = 5
print(point)
