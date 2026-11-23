# typy-test: TYP-28
# typy-expect: accepted
from typing import Literal

type Mode = Literal["r", "w"]


class File:
    def __init__(self, mode: Mode = "r") -> None:
        self.mode: Mode = mode


def describe(mode: Mode) -> str:
    match mode:
        case "r":
            return "read"
        case "w":
            return "write"


def parse(text: str) -> Mode | None:
    if text in ("r", "w"):
        return text
    return None


def level(value: Literal[-1, 0, 1], flag: Literal[True, False]) -> int:
    if value == -1 or flag == 0:
        return 0
    return value


handle = File("w")
print(describe(handle.mode), describe("r"), parse("w"), parse("x"))
print(level(1, True), level(-1, True), level(1, False))
print(Literal["r", "w"], Literal[1, Literal[2]] == Literal[2, 1])
