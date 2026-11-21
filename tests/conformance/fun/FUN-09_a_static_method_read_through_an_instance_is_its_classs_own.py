# typy-test: FUN-09
# typy-expect: accepted
from collections.abc import Callable


class Box:
    @staticmethod
    def double(value: int) -> int:
        return value * 2


class BigBox(Box):
    @staticmethod
    def double(value: int) -> int:
        return value * 20


class Small(Box):
    pass


def pick(flag: bool) -> Box:
    if flag:
        return BigBox()
    return Small()


first: Callable[[int], int] = pick(True).double
second: Callable[[int], int] = pick(False).double
print(first(1), second(1), Box.double(1), BigBox.double(1))
print(first is BigBox.double, second is Box.double, first == second)
