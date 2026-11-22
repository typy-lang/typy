# typy-test: EXP-22
# typy-expect: accepted
from collections.abc import Callable


def total(first: int, second: int) -> int:
    return first + second


class Meter:
    def scale(self, value: int, by: int) -> int:
        return value * by


def run(function: Callable[[int, int], int], values: list[int]) -> str:
    try:
        return str(function(*values))
    except TypeError as error:
        return str(error)


def outer() -> Callable[[int, int], int]:
    def inner(a: int, b: int) -> int:
        return a - b

    return inner


print(run(total, [1, 2]), run(total, [1, 2, 3]), run(Meter().scale, [3, 4]))
print(run(Meter().scale, [3, 4, 5]), run(outer(), [5, 2]), run(outer(), [1, 2, 3]))
print(run(lambda a, b: a * b, [6, 7]), run(lambda a, b: a * b, [6, 7, 8]))
pair = (4, 5)
f: Callable[[int, int], int] = total
print(f(1, *[2]), f(*pair), f(*(i for i in range(2))), f(*{9: 0, 1: 0}))
size: Callable[[str], int] = len
try:
    size(*["a", "b"])
except TypeError as error:
    print(error)
