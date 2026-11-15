# typy-test: TYP-14, TYP-07
# typy-expect: accepted
class Shape:
    def __init__(self, name: str) -> None:
        self.name = name


class Circle(Shape):
    pass


def first(value: str | bytes) -> str:
    item = value[0]
    return repr(item) + " " + type(item).__name__


def widen(value: int | str) -> int | str | None:
    return value


def wider(value: str | bytes) -> int | str | bytes:
    return value


def more(value: int | str) -> int | float | str:
    return value


def label(value: int | str | None) -> str:
    if value is None:
        return "none"
    if isinstance(value, str):
        return "str " + value
    return "int " + str(value * 2)


def shape_name(value: Shape | int) -> str:
    if isinstance(value, Circle):
        return "circle " + value.name
    if isinstance(value, Shape):
        return "shape " + value.name
    return "number " + str(value)


def flag(value: bool | str) -> str:
    if isinstance(value, bool):
        return "flag " + str(not value)
    return "text " + value


def point(value: tuple[int, int] | str) -> str:
    if isinstance(value, tuple):
        return "point " + str(value[0] + value[1])
    return "name " + value


print(first("ab"), first(b"cd"))
print(widen(3), widen("x"), wider(b"y"), wider("z"), more(5), more("w"))
print(label(None), label("a"), label(4))
print(shape_name(Circle("c")), shape_name(Shape("s")), shape_name(7))
print(flag(True), flag("t"), point((1, 2)), point("p"))
pairs: list[tuple[str, int | str]] = [("a", 1), ("b", "x")]
print(pairs, pairs[1][1])
maybe: int | str | None = None
print(maybe, maybe is None)
maybe = 5
print(maybe, maybe is None, isinstance(maybe, int))
