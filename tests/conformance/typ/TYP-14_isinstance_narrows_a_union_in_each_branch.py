# typy-test: TYP-14, TYP-07
# typy-expect: accepted
class Shape:
    def __init__(self, name: str) -> None:
        self.name = name


class Circle(Shape):
    def __init__(self, radius: float) -> None:
        super().__init__("circle")
        self.radius = radius


class Square(Shape):
    def __init__(self, side: float) -> None:
        super().__init__("square")
        self.side = side


def size(value: str | list[int] | dict[str, int]) -> int:
    if isinstance(value, str):
        return len(value) + 100
    elif isinstance(value, list):
        return sum(value)
    return len(value) * 1000


def measure(shape: Circle | Square | None) -> float:
    if shape is None:
        return -1.0
    if isinstance(shape, Circle):
        return shape.radius
    return shape.side


def label(value: str | bytes | None) -> str:
    if not isinstance(value, bytes):
        if value is None:
            return "nothing"
        return value.upper()
    return value.decode()


print(size("abc"), size([1, 2, 3]), size({"a": 1, "b": 2}))
print(measure(Circle(2.0)), measure(Square(3.0)), measure(None))
print(label("ab"), label(b"cd"), label(None))
print(isinstance(Circle(1.0), Shape), isinstance("x", (list, str)))
