# typy-test: FUN-04
# typy-expect: accepted
from typing import Callable


class Point:
    def __init__(self, x: int) -> None:
        self.x = x


def kinds() -> list[Callable[[], str]]:
    ratio = 0.5
    flag = False
    pair = (1, "a")
    maybe: int | None = None
    point = Point(1)
    readers = [lambda_free(lambda_read(ratio, flag, pair, maybe, point))]

    def read() -> str:
        return str(ratio) + str(flag) + str(pair) + str(maybe) + str(point.x)
    ratio, flag, pair, maybe, point = 2.5, True, (2, "b"), 3, Point(9)
    return [read]


def lambda_read(ratio: float, flag: bool, pair: tuple[int, str], maybe: int | None, point: Point) -> str:
    return str(ratio) + str(flag) + str(pair) + str(maybe) + str(point.x)


def lambda_free(text: str) -> Callable[[], str]:
    def give() -> str:
        return text
    return give


def shared() -> tuple[Callable[[], int], Callable[[], int]]:
    value = 1

    def first() -> int:
        return value

    def second() -> int:
        return value * 10
    value = 4
    return (first, second)


def unpacked() -> list[str]:
    readers: list[Callable[[], str]] = []
    for index, word in enumerate(["a", "b"]):
        def read() -> str:
            return str(index) + word
        readers.append(read)
    return [each() for each in readers]


def walrus() -> int:
    total = 0
    values = [(last := n) for n in range(4)]

    def read() -> int:
        return last + total + len(values)
    total = 100
    return read()


def recursive(depth: int) -> int:
    seen = depth

    def read() -> int:
        return seen
    seen = depth * 2
    return read() + (recursive(depth - 1) if depth > 0 else 0)


def comprehension() -> list[int]:
    scale = 1

    def scaled() -> list[int]:
        return [n * scale for n in range(3)]
    scale = 3
    return scaled()


print([each() for each in kinds()])
first, second = shared()
print(first(), second(), unpacked(), walrus(), recursive(3), comprehension())
