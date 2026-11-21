# typy-test: FUN-08
# typy-expect: accepted
import math
import sys
from collections.abc import Callable
from math import sqrt as root

floor_of: Callable[[float], int] = math.floor
square_root: Callable[[float], float] = root
limit: Callable[[], int] = sys.getrecursionlimit
print(sorted([2.5, -1.5, 0.5], key=math.fabs), floor_of(2.7), square_root(9.0))
print(limit() > 0, floor_of, square_root, square_root is math.sqrt, type(floor_of))


def measure() -> int:
    import math as numbers

    floor_of: Callable[[float], int] = numbers.floor
    return floor_of(2.5)


print(measure())
