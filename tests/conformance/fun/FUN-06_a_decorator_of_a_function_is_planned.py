# typy-test: FUN-06
# typy-expect: rejected E0201 at 10:2
from typing import Callable


def trace(function: Callable[[int], int]) -> Callable[[int], int]:
    return function


@trace
def double(value: int) -> int:
    return value * 2


print(double(4))
