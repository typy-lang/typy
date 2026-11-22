# typy-test: TYP-18
# typy-expect: accepted
from typing import Optional


class Box:
    pass


type Score = int
type Pair[T] = tuple[T, T]
type Plain = int | None
type Boxes = list[Box]
type Again = Score
type Nothing = None


def make() -> list[str]:
    type Local = list[int]
    type Wrapped[T] = dict[str, T]
    return [Local.__name__, Local.__module__, str(Local.__value__), str(Wrapped.__type_params__)]


print(Score.__value__, Pair.__value__, Plain.__value__, Boxes.__value__, Nothing.__value__)
print(Score.__type_params__, Pair.__type_params__, Again.__value__ is Score)
print(Boxes.__value__ == list[Box], Pair.__type_params__[0] == Pair.__type_params__[0])
print(make(), Pair[int] == Pair[int], Pair[int] == Pair[str], {Pair[str]: 1}[Pair[str]])
