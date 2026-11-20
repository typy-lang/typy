# typy-test: EXP-17
# typy-expect: accepted
from collections.abc import Iterator


def numbers() -> Iterator[int]:
    yield 7
    yield 8


a = [1, 2]
b = (3, 4)
print([*a, *b, 5], (*a, 6), {*a, *b}, [*range(2)], (*a,))
words = [*"xy", "z"]
print(words, [*numbers(), *a], sorted({*numbers()}))
d = {"x": 1}
e = {"y": 2, "x": 3}
print({**d, **e, "z": 4}, {"x": 0, **d})
nested = [[*a], [*b]]
print(nested)
optional: list[int | None] = [None, *a]
print(optional)
empty: list[int] = []
print([*empty], (*empty,), {**d})
