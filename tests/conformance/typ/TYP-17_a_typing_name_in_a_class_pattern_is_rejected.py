# typy-test: TYP-17
# typy-expect: rejected E0401 at 8:14
from typing import Iterator


def kind(value: list[int] | Iterator[int]) -> str:
    match value:
        case Iterator():
            return "iterator"
        case _:
            return "other"
