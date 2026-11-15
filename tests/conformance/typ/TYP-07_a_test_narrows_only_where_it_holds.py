# typy-test: TYP-07
# typy-expect: rejected E0414 at 16:18
class Shape:
    pass


class Circle(Shape):
    def __init__(self, radius: float) -> None:
        self.radius = radius


def radius(shape: Shape) -> float:
    if isinstance(shape, Circle):
        print(shape.radius)
    shape = Shape()
    return shape.radius
