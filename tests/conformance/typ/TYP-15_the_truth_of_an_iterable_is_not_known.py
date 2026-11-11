# typy-test: TYP-15, EXP-10
# typy-expect: rejected E0304 at 7:16
from collections.abc import Iterable


def empty(values: Iterable[int]) -> bool:
    return not values
