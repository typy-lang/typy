# typy-test: FUN-05
# typy-expect: rejected E0402 at 10:37
from typing import Callable


def scale(value: float) -> float:
    return value * 2.0


converter: Callable[[int], float] = scale
