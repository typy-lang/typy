# typy-test: FUN-04
# typy-expect: accepted
from typing import Callable


class Point:
    def __init__(self, x: int) -> None:
        self.x = x


def keep_point() -> Callable[[], int]:
    point = Point(42)

    def read() -> int:
        return point.x
    return read


def shared_list() -> tuple[Callable[[int], None], Callable[[], list[int]]]:
    items: list[int] = []

    def add(value: int) -> None:
        items.append(value)

    def show() -> list[int]:
        return items
    return (add, show)


def kinds(flag: bool, ratio: float, pair: tuple[int, str], maybe: int | None) -> Callable[[], str]:
    def describe() -> str:
        return str(flag) + " " + str(ratio) + " " + str(pair) + " " + str(maybe)
    return describe


def options() -> int:
    def scale(value: int, *, by: int = 2) -> int:
        return value * by
    return scale(3) + scale(3, by=10)


def calls_sibling() -> int:
    def base() -> int:
        return 7

    def twice() -> int:
        return base() * 2
    helpers = [base, twice]
    return sum([helper() for helper in helpers])


def fails() -> Callable[[int], int]:
    def check(value: int) -> int:
        if value < 0:
            raise ValueError("negative " + str(value))
        return value
    return check


read = keep_point()
print(read(), read())
add, show = shared_list()
add(1)
add(2)
print(show())
print(kinds(True, 1.5, (1, "a"), None)(), kinds(False, 0.0, (2, "b"), 3)())
print(options(), calls_sibling())
checker = fails()
try:
    checker(-5)
except ValueError as error:
    print("ValueError:", error)
print(checker(5))
