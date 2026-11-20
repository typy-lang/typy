# typy-test: SMT-02, EXP-17
# typy-expect: accepted
# typy-python: 3.15
from collections.abc import Iterator


def gen(n: int) -> Iterator[int]:
    yield from range(n)


def endless() -> Iterator[int]:
    value = 0
    while True:
        yield value
        value += 1


class Point:
    def __init__(self) -> None:
        self.x = 0
        self.y = 0


def report(label: str, error: ValueError) -> None:
    print(label, error)


try:
    a, b = [1, 2, 3]
except ValueError as error:
    report("list", error)
try:
    a, b = gen(3)
except ValueError as error:
    report("generator", error)
try:
    c, d = "abc"
except ValueError as error:
    report("str", error)
try:
    a, b = range(3)
except ValueError as error:
    report("range", error)
try:
    a, b = {1: "x", 2: "y", 3: "z"}
except ValueError as error:
    report("dict", error)
try:
    a, b = endless()
except ValueError as error:
    report("endless", error)
try:
    a, *rest, b = gen(1)
except ValueError as error:
    report("starred generator", error)
point = Point()
point.x, point.y = [3, 4]
print(point.x, point.y)
items = [0, 0, 0]
items[0], *middle, items[2] = range(5)
print(items, middle)
(e, f), *others = [[1, 2], [3, 4], [5, 6]]
print(e, f, others)
for key, *values in [["k", "v1", "v2"], ["j"]]:
    print(key, values)
pairs = {"a": 1}.items()
for name, number in pairs:
    print(name, number)
squares = [x * y for x, y in [[1, 2], [3, 4]]]
print(squares)
