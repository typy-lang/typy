# typy-test: TYP-24
# typy-expect: accepted
# A cast of an optional to the type it holds tests only that the value is not `None`.
from typing import cast


def find(items: list[int], wanted: int) -> int | None:
    for position, item in enumerate(items):
        if item == wanted:
            return position
    return None


position = cast(int, find([3, 1, 4], 4))
print(position + 1)
