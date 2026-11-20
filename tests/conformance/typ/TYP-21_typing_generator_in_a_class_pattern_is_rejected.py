# typy-test: TYP-21
# typy-expect: rejected E0401 at 13:14
import typing
from collections.abc import Iterator


def numbers() -> Iterator[int]:
    yield 1


def kind(value: Iterator[int]) -> str:
    match value:
        case typing.Generator():
            return "generator"
        case _:
            return "other"


print(kind(numbers()))
