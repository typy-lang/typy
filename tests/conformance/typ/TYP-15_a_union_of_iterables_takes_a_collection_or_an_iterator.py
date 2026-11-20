# typy-test: TYP-15, TYP-14
# typy-expect: accepted
from collections.abc import Iterable, Iterator


def numbers() -> Iterator[int]:
    yield 1


def kind(values: Iterable[int] | Iterable[str]) -> str:
    return type(values).__name__


def mixed(values: Iterable[int] | Iterator[str]) -> str:
    return type(values).__name__


def either(values: Iterable[int] | int) -> str:
    return type(values).__name__


print(kind([1, 2]), kind(["a"]), kind((1, 2)), kind("ab"), kind({1: 2}))
print(kind(numbers()), kind(number for number in [1]))
print(mixed([1]), mixed(iter("ab")), mixed((1, 2)))
print(either((1, 2)), either([1]), either(3))
