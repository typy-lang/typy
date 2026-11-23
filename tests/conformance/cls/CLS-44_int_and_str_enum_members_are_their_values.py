# typy-test: CLS-44
# typy-expect: accepted
# A member of an IntEnum or a StrEnum shows, compares, hashes and calculates as its value,
# and keeps its own repr.
from enum import IntEnum, StrEnum, auto


class Level(IntEnum):
    LOW = auto()
    HIGH = 10
    TOP = auto()
    ZERO = 0


class Mode(StrEnum):
    READ = auto()
    WRITE = "w"


def double(n: int) -> int:
    return n * 2


def shout(text: str) -> str:
    return text.upper() + "!"


print(repr(Level.LOW), Level.LOW, f"{Level.TOP}", f"{Level.HIGH:03}")
print(Level.LOW + 1, Level.TOP - Level.LOW, -Level.HIGH, Level.HIGH // 3)
print(Level.LOW == 1, Level.HIGH > 5, sorted([Level.TOP, Level.LOW, Level.HIGH]))
print(double(Level.HIGH), [10, 20, 30][Level.LOW], bool(Level.ZERO), hex(Level.HIGH))
print(sorted({Level.HIGH, Level.LOW}), list(range(Level.LOW)), divmod(Level.HIGH, 4))
print(repr(Mode.READ), Mode.READ, f"{Mode.WRITE:>3}|", Mode.READ == "read")
print(shout(Mode.WRITE), Mode.READ.upper(), Mode.READ + "!", len(Mode.READ))
print(list(Level), Level(10), Level["TOP"], Mode("w"), Mode.WRITE in Mode)
