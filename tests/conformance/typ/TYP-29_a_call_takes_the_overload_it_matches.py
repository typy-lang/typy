# typy-test: TYP-29
# typy-expect: accepted
from typing import Literal, overload


@overload
def double(value: int) -> int: ...
@overload
def double(value: str) -> str: ...
def double(value: int | str) -> int | str:
    if isinstance(value, int):
        return value * 2
    return value + value


@overload
def read(mode: Literal["r"]) -> str: ...
@overload
def read(mode: Literal["rb"]) -> bytes: ...
def read(mode: Literal["r", "rb"]) -> str | bytes:
    if mode == "r":
        return "text"
    return b"data"


class Shelf:
    def __init__(self, items: list[int]) -> None:
        self.items = items

    @overload
    def get(self, index: int) -> int: ...
    @overload
    def get(self, index: slice) -> list[int]: ...
    def get(self, index: int | slice) -> int | list[int]:
        if isinstance(index, int):
            return self.items[index]
        return self.items[index]

    @overload
    def find(self, item: int) -> int | None: ...
    @overload
    def find(self, item: int, default: int) -> int: ...
    def find(self, item: int, default: int | None = None) -> int | None:
        for position, each in enumerate(self.items):
            if each == item:
                return position
        return default


shelf = Shelf([3, 1, 4])
number: int = double(21)
text: str = double("ab")
first: int = shelf.get(0)
rest: list[int] = shelf.get(slice(1, 3))
at: int = shelf.find(9, -1)
print(number + 1, text.upper(), read("r").upper(), read("rb"))
print(first * 2, rest, at, shelf.find(4), shelf.find(5))
