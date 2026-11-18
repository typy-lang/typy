# typy-test: FUN-05
# typy-expect: accepted
from collections.abc import Callable as Function
from typing import Callable


def double(value: int) -> int:
    return value * 2


def square(value: int) -> int:
    return value * value


def chooser(squares: bool) -> Callable[[int], int]:
    return square if squares else double


def apply_all(functions: list[Callable[[int], int]], value: int) -> list[int]:
    return [function(value) for function in functions]


def twice(function: Function[[int], int], value: int) -> int:
    return function(function(value))


class Counter:
    def __init__(self, step: Callable[[int], int]) -> None:
        self.step = step
        self.total = 1

    def advance(self) -> int:
        self.total = self.step(self.total)
        return self.total


operations = [double, square]
print(apply_all(operations, 3), twice(square, 3), chooser(True)(5), chooser(False)(5))
names: dict[Callable[[int], int], str] = {double: "double", square: "square"}
print(names[chooser(True)], len({double, square, double}), double in names)
counter = Counter(double)
print(counter.advance(), counter.advance(), counter.step is double)
same = double
print(same is double, same == double, same != square, operations.index(square))
print(type(double), type(double) is type(square))
print(repr(double).startswith("<function double at 0x"), str(square) == repr(square))
operations.reverse()
print(apply_all(operations, 4), operations == [square, double])
