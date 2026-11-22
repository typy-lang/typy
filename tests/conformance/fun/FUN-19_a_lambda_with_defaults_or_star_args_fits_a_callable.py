# typy-test: FUN-19
# typy-expect: accepted
from collections.abc import Callable


def apply(function: Callable[[int, int], int], left: int, right: int) -> int:
    return function(left, right)


def once(function: Callable[[int], int], value: int) -> int:
    return function(value)


product: Callable[[int], int] = lambda value, by=4: value * by
print(product(3), repr(product).split(" at ")[0])
print(apply(lambda *values: sum(values), 6, 7), apply(lambda first, *rest: first - len(rest), 5, 1))
print(once(lambda value, *, step=5: value + step, 1))
print(sorted(["bb", "a", "ccc"], key=lambda text, pad=10: pad - len(text)))
