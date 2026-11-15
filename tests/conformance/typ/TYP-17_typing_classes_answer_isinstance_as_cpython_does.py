# typy-test: TYP-17
# typy-expect: accepted
import typing
from collections.abc import Iterable, Iterator
from typing import Optional, Union, List


class Bag:
    def __iter__(self) -> Iterator[int]:
        return iter([1])


class Walker:
    def __iter__(self) -> "Walker":
        return self

    def __next__(self) -> int:
        raise StopIteration


class Plain:
    pass


class Grown(Plain):
    def __iter__(self) -> Iterator[int]:
        return iter([2])


def describe(label: str, iterable: bool, iterator: bool) -> None:
    print(label, iterable, iterator)


numbers = [1, 2]
describe("list", isinstance(numbers, Iterable), isinstance(numbers, Iterator))
describe("str", isinstance("s", typing.Iterable), isinstance("s", typing.Iterator))
describe("keys", isinstance({1: 2}.keys(), Iterable), isinstance({1: 2}.keys(), Iterator))
describe("range", isinstance(range(3), Iterable), isinstance(range(3), Iterator))
describe("iter", isinstance(iter(numbers), Iterable), isinstance(iter(numbers), Iterator))
describe("int", isinstance(3, Iterable), isinstance(3, Iterator))
describe("Bag", isinstance(Bag(), Iterable), isinstance(Bag(), Iterator))
describe("Walker", isinstance(Walker(), Iterable), isinstance(Walker(), Iterator))
plains: list[Plain] = [Plain(), Grown()]
for plain in plains:
    describe(type(plain).__name__, isinstance(plain, Iterable), isinstance(plain, Iterator))


def kind(value: int | str | list[int] | None) -> str:
    if isinstance(value, Optional[int]):
        return "maybe int"
    if isinstance(value, Union[str, List]):
        return "str or list"
    return "other"


print(kind(1), kind(None), kind("a"), kind([1]))


def shape(value: int | list[int] | Bag) -> str:
    match value:
        case Iterable():
            return "iterable"
        case _:
            return "number"


print(shape(1), shape([1]), shape(Bag()), isinstance((1,), typing.Tuple), isinstance({1}, typing.Set))
