# typy-test: TYP-28
# typy-expect: accepted
# A variable narrowed to constants by `==`, `in` or `not in` still holds a value of its own
# type, so an operation on it runs once, with no test of which constant it is.
from typing import Literal


def step(count: int) -> int:
    if count not in (0, 1, 3):
        return -1
    return count * 10 + 1


def shout(text: str) -> str:
    if text in ("r", "w"):
        return text.upper() + text[0]
    return "?"


def ratio(scale: float) -> float:
    if scale == 0.5 or scale == 2.0:
        return scale / 2
    return 1.0


def twice(value: Literal[0, 7, "a"]) -> int | str:
    if value != 0:
        return value * 2
    return value


print(step(0), step(3), step(2), shout("w"), shout("x"), ratio(2.0), ratio(3.0))
print(twice(0), twice(7), twice("a"))
