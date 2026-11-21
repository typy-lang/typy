# typy-test: EXP-19
# typy-expect: accepted
from collections.abc import Callable


def apply(function: Callable[[int, int], int], pair: tuple[int, int]) -> int:
    return function(*pair)


pair = (7, 3)
print(max(*pair), min(*(4, 9, 2)), round(*(2.675, 2)))
print(apply(lambda a, b: a - b, pair), *pair, **{"sep": "-", "end": "!\n"})
error = ValueError(*("bad", 2))
print(error.args, len(*("abc",)), abs(*(-5,)))
words = ("b", "a")
print(sorted(words), "-".join(words), str(*(12,)))
