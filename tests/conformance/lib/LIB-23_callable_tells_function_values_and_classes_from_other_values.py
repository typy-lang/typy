# typy-test: LIB-23
# typy-expect: accepted
from collections.abc import Callable


def double(value: int) -> int:
    return value * 2


class Box:
    pass


maybe: Callable[[int], int] | None = None
print(callable(double), callable(len), callable(int), callable(Box), callable(3), callable(None))
print(callable("text"), callable(maybe), callable([double][0]))
maybe = double

print(callable(maybe), callable(Box()), callable(str.upper), callable(abs))
