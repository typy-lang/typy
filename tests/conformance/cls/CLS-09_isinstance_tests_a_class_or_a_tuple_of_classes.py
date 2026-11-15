# typy-test: CLS-09, TYP-07
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


class Disc(Circle):
    pass


def area(shape: Shape) -> float:
    if isinstance(shape, Circle):
        return 3.0 * shape.radius * shape.radius
    if not isinstance(shape, Square):
        return 0.0
    return shape.side * shape.side


def describe(shape: Shape | None) -> str:
    if isinstance(shape, Circle):
        return "round " + str(shape.radius)
    if shape is None:
        return "nothing"
    return shape.name


shapes: list[Shape] = [Circle(1.0), Square(2.0), Shape("blob"), Disc(3.0)]
for shape in shapes:
    print(area(shape), describe(shape), isinstance(shape, (Circle, Square)),
          isinstance(shape, Disc), isinstance(shape, Shape), isinstance(shape, object),
          isinstance(shape, int))
print(describe(None), isinstance(None, object), isinstance(3, int), isinstance(True, int),
      isinstance("x", (int, str)), isinstance([1], list), isinstance(2.5, int))
try:
    raise KeyError("k")
except LookupError as error:
    print(isinstance(error, KeyError), isinstance(error, IndexError), isinstance(error, Exception))
maybe: Shape | None = Square(1.5)
if isinstance(maybe, Square):
    print("side", maybe.side)
