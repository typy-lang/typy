# typy-test: FUN-19
# typy-expect: accepted
from collections.abc import Callable


def apply(function: Callable[[int, int], int], left: int, right: int) -> int:
    return function(left, right)


def once(function: Callable[[int], int], value: int) -> int:
    return function(value)


def keep(function: Callable[[int], int]) -> Callable[[int], int]:
    return function


def outer(base: int) -> list[Callable[[int], int]]:
    made: list[Callable[[int], int]] = []
    for step in range(3):

        def add(value: int, more: int = 1) -> int:
            return value + more

        made.append(add)

    def total(*values: int, **named: int) -> int:
        return sum(values) + base + len(named)

    def run() -> int:
        return apply(total, 2, 3)

    first = keep(total)
    second = keep(total)
    print(run(), once(total, 4), first is second, first == second, {first: 1}[second])
    print(repr(first).split(" at ")[0], made[0] is made[1])
    made.append(total)
    return made


print([each(1) for each in outer(10)])
print(sorted([3, 1, 2], key=outer(0)[3]))
