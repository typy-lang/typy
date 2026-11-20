# typy-test: TYP-17
# typy-expect: accepted
from collections.abc import Iterator


def kind(value: list[int] | Iterator[int] | int) -> str:
    match value:
        case Iterator():
            return "iterator"
        case list():
            return "list"
        case _:
            return "number"


print(kind(iter([1])), kind([1]), kind(2))
