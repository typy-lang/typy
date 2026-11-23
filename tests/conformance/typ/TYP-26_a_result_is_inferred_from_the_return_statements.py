# typy-test: TYP-26
# typy-expect: accepted
# A function of the module without a result annotation returns what its `return`
# statements give: one type, a union with `None`, or `None` when it returns no value.
from collections.abc import Callable


SCALE = 3


def double(value: int):
    return value * 2


def scaled(value: int):
    return double(value) * SCALE


def label(value: int):
    if value > 10:
        return "big"
    return "small"


def maybe(value: int):
    if value > 0:
        return value
    return None


def nothing(value: int):
    print("nothing", value)


def describe(values: list[int]):
    return [label(value) for value in values]


RESULT = scaled(4)
print(RESULT + 1, label(20), maybe(-1), maybe(5), nothing(1), describe([1, 50]))
print(type(RESULT).__name__)


def twice(function: Callable[[int], int]) -> Callable[[int], int]:
    def run(n: int) -> int:
        return function(function(n))

    return run


@twice
def increment(n: int):
    return n + 1


print(increment(3))


def first(values: list[int]):
    for value in values:
        return value


print(first([]), first([7]))
