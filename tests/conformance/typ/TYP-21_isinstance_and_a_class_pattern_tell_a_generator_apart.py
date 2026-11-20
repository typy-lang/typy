# typy-test: TYP-21, LIB-20
# typy-expect: accepted
import typing
from collections.abc import Generator, Iterable, Iterator


def numbers() -> Generator[int, None, None]:
    yield 1
    yield 2


def count() -> Iterator[int]:
    yield 3


class Steps:
    def __iter__(self) -> "Steps":
        return self

    def __next__(self) -> int:
        raise StopIteration


class Fake:
    def __iter__(self) -> "Fake":
        return self

    def __next__(self) -> int:
        raise StopIteration

    def send(self, value: None) -> int:
        raise StopIteration

    def throw(self, error: Exception) -> int:
        raise error

    def close(self) -> None:
        pass


def kind(value: Iterable[int]) -> str:
    if isinstance(value, Generator):
        return "generator"
    if isinstance(value, Iterator):
        return "iterator"
    return "iterable"


def matched(value: Iterable[int] | int) -> str:
    match value:
        case Generator():
            return "generator"
        case Iterator():
            return "iterator"
        case _:
            return "other"


def first(values: Iterable[int]) -> int:
    if isinstance(values, Iterator):
        return next(values)
    return -1


print(kind(numbers()), kind(count()), kind(iter([1])), kind([1]), kind(Steps()), kind(Fake()))
print(matched(numbers()), matched(iter([1])), matched(3), matched(Steps()), matched([1]))
print(isinstance(numbers(), typing.Generator), isinstance(iter([1]), typing.Generator))
print(first(numbers()), first([5]), first(iter([7])))
