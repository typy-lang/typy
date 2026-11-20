# typy-test: CLS-27
# typy-expect: rejected E0201 at 6:13
import typing


class Point(typing.NamedTuple):
    x: int
    y: int


print(Point.__name__)
