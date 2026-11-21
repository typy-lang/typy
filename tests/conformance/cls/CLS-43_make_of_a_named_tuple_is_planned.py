# typy-test: CLS-43
# typy-expect: rejected E0201 at 10:13
from typing import NamedTuple


class Point(NamedTuple):
    x: int


print(Point._make([1]))
