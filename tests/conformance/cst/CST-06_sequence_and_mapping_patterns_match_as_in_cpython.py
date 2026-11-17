# typy-test: CST-06
# typy-expect: accepted
def shape(values: list[int]) -> str:
    match values:
        case []:
            return "empty"
        case [only]:
            return f"one {only}"
        case [first, 0]:
            return f"{first} then zero"
        case [first, *middle, last]:
            return f"{first}..{last} around {middle}"
    return "unreachable"


def point(pair: tuple[int, int]) -> str:
    match pair:
        case (0, 0):
            return "origin"
        case (x, 0) | (0, x):
            return f"axis {x}"
        case (x, y):
            return f"at {x} {y}"


def maybe(values: list[str] | None) -> str:
    match values:
        case None:
            return "nothing"
        case ["go", where]:
            return f"go to {where}"
        case ["say", *words]:
            return " ".join(words)
        case _:
            return "unknown"


def nested(grid: list[list[int]]) -> int:
    match grid:
        case [[a, b], [c, d]]:
            return a * d - b * c
        case [[*row], *_]:
            return len(row)
        case _:
            return -1


def config(settings: dict[str, int]) -> str:
    match settings:
        case {"width": 0}:
            return "no width"
        case {"width": width, "height": height}:
            return f"{width}x{height}"
        case {"width": width, **others}:
            return f"width {width} and {sorted(others)}"
        case {}:
            return "empty or other"


def mixed(value: int | list[int] | dict[str, int]) -> str:
    match value:
        case [x, y]:
            return f"pair {x + y}"
        case {"k": v}:
            return f"k {v}"
        case _:
            return "other"


for values in [[], [5], [3, 0], [1, 2, 3, 4], [7, 8]]:
    print(shape(values))
for pair in [(0, 0), (4, 0), (0, 6), (2, 3)]:
    print(point(pair))
for words in [None, ["go", "home"], ["say", "hello", "there"], ["say"], ["x", "y", "z"]]:
    print(maybe(words))
print(nested([[1, 2], [3, 4]]), nested([[1, 2, 3]]), nested([]))
for settings in [{"width": 0}, {"width": 3, "height": 4}, {"width": 5, "depth": 1, "a": 2}, {"x": 1}]:
    print(config(settings))
triple = (1, "two", 3.0)
match triple:
    case (a, *rest):
        print(a, rest)
match range(5):
    case [0, 1, *others]:
        print("range", others)
