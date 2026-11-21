# typy-test: FUN-19
# typy-expect: rejected E0409 at 10:13
from collections.abc import Callable


def run(function: Callable[[int, str], int]) -> int:
    return function(1, "a")


run(lambda *values: len(values))
