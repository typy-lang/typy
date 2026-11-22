# typy-test: EXP-21
# typy-expect: rejected E0201 at 12:9
from collections.abc import Callable


def total(first: int, second: int) -> int:
    return first + second


f: Callable[[int, int], int] = total
options = {"first": 1, "second": 2}
print(f(**options))
