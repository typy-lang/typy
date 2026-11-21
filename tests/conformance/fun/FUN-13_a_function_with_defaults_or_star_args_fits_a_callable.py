# typy-test: FUN-13
# typy-expect: accepted
from collections.abc import Callable


def total(*values: int) -> int:
    return sum(values)


def scale(value: int, by: int = 2) -> int:
    return value * by


def pick(first: int, *rest: int, **named: int) -> int:
    return first + len(rest) + len(named)


def apply(function: Callable[[int, int], int], left: int, right: int) -> int:
    return function(left, right)


def once(function: Callable[[int], int], value: int) -> int:
    return function(value)


print(apply(total, 2, 3), once(scale, 3), apply(pick, 10, 20), once(total, 7))
twice: Callable[[int], int] = scale
summed: Callable[[int, int, int], int] = total
print(twice(5), summed(1, 2, 3), twice is scale, twice == scale, summed is total)
