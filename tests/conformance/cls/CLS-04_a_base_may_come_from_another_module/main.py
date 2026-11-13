# typy-test: CLS-04, MOD-01
# typy-expect: accepted
import shapes
from shapes import Shape


class Square(Shape):
    def __init__(self, side: int) -> None:
        self.name = "square"
        self.side = side


class Circle(shapes.Shape):
    pass


class BadSide(shapes.Problem):
    pass


every: list[Shape] = [Square(2), Circle("circle")]
for each in every:
    print(each.describe(), type(each).__name__)
try:
    raise BadSide("negative")
except ValueError as error:
    print("caught", error)
