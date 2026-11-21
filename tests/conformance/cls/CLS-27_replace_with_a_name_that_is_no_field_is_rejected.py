# typy-test: CLS-27
# typy-expect: rejected E0404 at 11:7
from typing import NamedTuple


class Point(NamedTuple):
    x: int
    y: int


print(Point(1, 2)._replace(z=3))
