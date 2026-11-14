# typy-test: LIB-06
# typy-expect: accepted
import typing
from collections.abc import Iterable
from typing import ClassVar, Final, Iterator, Protocol

LIMIT: Final = 3


class Counter:
    made: ClassVar[int] = 0


class Named(Protocol):
    def name(self) -> str: ...


class Item:
    def name(self) -> str:
        return "item"


def first(items: Iterable[int]) -> int:
    for item in items:
        return item
    return LIMIT


def describe(thing: Named) -> str:
    return thing.name()


def counted(values: list[int]) -> typing.Iterator[int]:
    return iter(values)


steps: Iterator[int] = iter([1, 2])
print(first([5, 6]), first(range(0)), next(steps), describe(Item()), list(counted([7])))
print(Counter.made, LIMIT)
