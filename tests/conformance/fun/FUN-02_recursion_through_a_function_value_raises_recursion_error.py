# typy-test: FUN-02
# typy-expect: accepted
from typing import Callable

step: Callable[[int], int] | None = None


def countdown(value: int) -> int:
    current = step
    if current is None or value == 0:
        return 0
    return current(value - 1) + 1


step = countdown
print(countdown(100))
try:
    countdown(100_000_000)
except RecursionError:
    print("RecursionError")
