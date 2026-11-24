# typy-test: CLS-44
# typy-expect: accepted
# Members, aliases, auto(), lookup by value and by name with CPython's errors, iteration,
# len, in and match.
from enum import Enum, auto


class Color(Enum):
    """Colours."""

    RED = 1
    GREEN = auto()
    BLUE = auto()
    CRIMSON = 1

    def label(self) -> str:
        return self.name.lower()


class Mode(Enum):
    READ = "r"
    WRITE = "w"
    RAW = b"raw"


def describe(color: Color) -> str:
    match color:
        case Color.RED:
            return "warm"
        case Color.GREEN | Color.BLUE:
            return "cool"
    return "none"


print(repr(Color.RED), Color.RED, Color.GREEN.name, Color.BLUE.value)
print(Color.CRIMSON is Color.RED, Color.RED == Color.GREEN, len(Color))
print(list(Color), [color.label() for color in Color])
print(Color(2), Color["CRIMSON"], Color.BLUE in Color)
print(Mode("w"), Mode(b"raw"), repr(Mode.READ), list(Mode))
print({Color.RED: "r"}[Color.CRIMSON], describe(Color.BLUE))
for color in Color:
    print(color, color.value)
try:
    Color(9)
except ValueError as error:
    print(error)
try:
    Mode("x")
except ValueError as error:
    print(error)
try:
    Color["PINK"]
except KeyError as error:
    print(repr(error))
