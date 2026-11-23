# typy-test: TYP-28
# typy-expect: rejected E0431 at 10:15
from typing import Literal


def open_as(mode: Literal["r", "w"]) -> str:
    return mode


print(open_as("a"))
