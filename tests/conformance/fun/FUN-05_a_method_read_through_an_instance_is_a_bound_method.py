# typy-test: FUN-05
# typy-expect: accepted
from typing import Callable


class Counter:
    def __init__(self, start: int) -> None:
        self.count = start

    def add(self, step: int) -> int:
        self.count += step
        return self.count

    def __repr__(self) -> str:
        return "Counter(" + str(self.count) + ")"


class Double(Counter):
    def add(self, step: int) -> int:
        self.count += 2 * step
        return self.count


class Triple(Double):
    pass


class Unit:
    @staticmethod
    def make(value: int) -> int:
        return value + 1000

    def show(self) -> str:
        return "unit"


def run(step: Callable[[int], int], times: int) -> int:
    total = 0
    for _ in range(times):
        total = step(1)
    return total


counter = Counter(0)
increment = counter.add
print(run(increment, 3), counter.count, repr(increment))
base: Counter = Triple(10)
bound = base.add
print(bound(1), repr(bound), bound == base.add, bound is base.add)
print(Double(0).add == Double(0).add, len({counter.add, counter.add, base.add}))
plain = Counter.add
print(plain(counter, 5), plain is Counter.add, repr(plain).startswith("<function Counter.add at"))
print(Unit.make(1), Unit().make(2), Unit.make is Unit().make, str(type(bound)))
shown = Unit().show
callbacks: list[Callable[[], str]] = [shown, Unit().show]
print([each() for each in callbacks], callbacks[0] == shown)


def keep() -> Callable[[int], int]:
    local = Counter(100)
    return local.add


later = keep()
print(later(1), later(1))
