# typy-test: FUN-19
# typy-expect: accepted
from collections.abc import Callable


def once(function: Callable[[int], int], value: int) -> int:
    return function(value)


def keep(function: Callable[[int], int]) -> Callable[[int], int]:
    return function


class Meter:
    def __init__(self, factor: int) -> None:
        self.factor = factor

    def scale(self, value: int, extra: int = 0) -> int:
        return value * self.factor + extra

    def total(self, *values: int) -> int:
        return sum(values) * self.factor

    def step(self, value: int, *, by: int = 7) -> int:
        return value + by

    @classmethod
    def make(cls, value: int, more: int = 100) -> int:
        return value + more

    @staticmethod
    def less(value: int, *rest: int) -> int:
        return value - sum(rest)


class Wide(Meter):
    def scale(self, value: int, extra: int = 0) -> int:
        return value * self.factor * 10 + extra


meter: Meter = Wide(2)
print(once(meter.scale, 3), once(Meter(2).scale, 3), once(meter.total, 4))
print(once(meter.step, 1), once(Meter.make, 1), once(meter.make, 2))
print(once(Meter.less, 9), once(meter.less, 9))
bound = keep(meter.scale)
print(repr(bound).split(" at ")[0], type(bound).__name__, bound == keep(meter.scale))
through_class: Callable[[Meter, int], int] = Meter.scale
print(through_class(Wide(3), 2), repr(through_class).split(" at ")[0])
