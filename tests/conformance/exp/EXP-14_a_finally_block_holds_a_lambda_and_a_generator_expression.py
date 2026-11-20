# typy-test: EXP-14, EXP-15
# typy-expect: accepted
from collections.abc import Callable


def run(flag: bool) -> int:
    try:
        if flag:
            return 1
    finally:
        print(sum(number for number in [1, 2]))
        double: Callable[[int], int] = lambda number: number * 2
        print(double(3))
    return 0


print(run(True), run(False))
