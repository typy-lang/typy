# typy-test: CLS-16
# typy-expect: accepted
from dataclasses import dataclass

type Numbers = list[int]


@dataclass(frozen=True)
class Pair:
    items: tuple[list[int], int]


@dataclass(frozen=True)
class Bag:
    numbers: Numbers


print(Pair(([1], 2)), Pair(([1], 2)) == Pair(([1], 2)))
print(Bag([3]), Bag([3]) == Bag([4]))
