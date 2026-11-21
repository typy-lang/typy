# typy-test: FUN-06
# typy-expect: rejected E0404 at 15:17
from collections.abc import Callable


def trace(function: Callable[[int], int]) -> Callable[[int], int]:
    return function


@trace
def increment(value: int) -> int:
    return value + 1


print(increment(value=1))
