# typy-test: CLS-27
# typy-expect: rejected E0404 at 8:8
from typing import NamedTuple


class Point(NamedTuple):
    x: int = 0
    y: int


print(Point(1, 2))
