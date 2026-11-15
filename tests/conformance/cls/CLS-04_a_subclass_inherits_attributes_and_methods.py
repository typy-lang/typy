# typy-test: CLS-04
# typy-expect: accepted
class Shape:
    name: str

    def __init__(self, name: str) -> None:
        self.name = name

    def describe(self) -> str:
        return "shape " + self.name

    def __repr__(self) -> str:
        return "Shape(" + repr(self.name) + ")"


class Circle(Shape):
    name: str
    radius: float

    def __init__(self, radius: float) -> None:
        self.name = "circle"
        self.radius = radius

    def area(self) -> float:
        return 3.0 * self.radius * self.radius


class Ring(Circle):
    def __init__(self, radius: float, width: float) -> None:
        self.name = "ring"
        self.radius = radius
        self.width = width


def show(shape: Shape) -> str:
    return shape.describe()


def name_of(shape: Shape | None) -> str:
    if shape is None:
        return "nothing"
    return shape.name


ring = Ring(2.0, 0.5)
print(show(ring), ring.area(), ring.width, ring)
shapes: list[Shape] = [Circle(1.0), Shape("plain"), ring]
for shape in shapes:
    print(shape.describe(), shape is ring, shape == ring)
maybe: Circle | None = Circle(3.0)
print(name_of(maybe), name_of(None), name_of(ring))
first: Shape = shapes[0]
first.name = "renamed"
print(shapes[0].name, type(first).__name__, shapes)
