# typy-test: TYP-14
# typy-expect: accepted
class Shape:
    def __init__(self, name: str) -> None:
        self.name = name

    def area(self) -> float:
        return 0.0


class Circle(Shape):
    def __init__(self, radius: float) -> None:
        super().__init__("circle")
        self.radius = radius

    def area(self) -> float:
        return 3.0 * self.radius * self.radius


class Square(Shape):
    def __init__(self, side: float) -> None:
        super().__init__("square")
        self.side = side

    def area(self) -> float:
        return self.side * self.side


def describe(value: str | list[int]) -> str:
    return "value " + str(value) + " " + repr(value)


def biggest(shapes: list[Circle | Square]) -> Circle | Square:
    best = shapes[0]
    for shape in shapes:
        if shape.area() > best.area():
            best = shape
    return best


print(describe("abc"))
print(describe([1, 2]))
things: list[str | list[int]] = ["a", [3], "b"]
print(things)
shape = biggest([Circle(1.0), Square(2.0)])
print(shape.name, shape.area())
maybe: str | bytes | None = None
print(maybe)
maybe = b"xy"
print(maybe, f"{maybe}")
