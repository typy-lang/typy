# typy-test: FUN-19
# typy-expect: accepted
from collections.abc import Callable
from typing import Protocol


class Scaled(Protocol):
    def scale(self, value: int, extra: int = 0) -> int: ...


class Meter:
    def scale(self, value: int, extra: int = 0) -> int:
        return value + extra + 100


class Gauge:
    def scale(self, value: int, extra: int = 5) -> int:
        return value * 2 + extra


def once(function: Callable[[int], int], value: int) -> int:
    return function(value)


items: list[Scaled] = [Meter(), Gauge()]
for item in items:
    adapted: Callable[[int], int] = item.scale
    print(once(item.scale, 1), adapted(2), repr(adapted).split(" at ")[0])
