# typy-test: TYP-18
# typy-expect: accepted
# typy-python: 3.15
from typing import Optional


class Holder:
    type Inner = str
    type Pairs[T] = list[tuple[T, T]]


type Pair[T] = tuple[T, T]
type Maybe = Optional[int]
type Scores = list[Pair[int]]


def make() -> list[object]:
    type Local = list[int]
    print(Local, Local.__value__)
    return [Local]


first = make()
second = make()
print(first[0] is second[0], first[0] is first[0], first[0] == second[0])
print(Holder.Inner, Holder.Inner.__value__, Holder.Pairs.__value__, Holder.Pairs.__type_params__)
print(Pair[int], Pair[int, str], Maybe.__value__, Scores.__value__)
