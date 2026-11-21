# typy-test: CLS-41
# typy-expect: rejected E0201 at 11:9
from typing import NamedTuple


class Point(NamedTuple):
    x: int


class Moved(Point):
    def __init__(self) -> None:
        pass


print(Moved(1))
