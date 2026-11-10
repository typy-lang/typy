# typy-test: CLS-12
# typy-expect: accepted
class Shape:
    def __init__(self, sides: int) -> None:
        self.sides = sides


def describe(value: type) -> str:
    return value.__name__ + " " + str(value)


shape = Shape(3)
maybe: Shape | None = None
print(type(shape), type(shape) is Shape, type(1) == int, type(1) != float)
print(describe(type(shape)), describe(int), type(maybe), type(maybe).__name__)
maybe = shape
print(type(maybe), type(type(shape)), [type(item) for item in [1, 2]] == [int, int])
names = {int: "number", str: "text", Shape: "shape"}
print(names[type("x")], names[type(shape)], len({int, str, int}))
print(type([1]), type((1, "a")), type({1}), type({1: 2}), type({1: 2}.keys()))
print(type(None), type(True), type(b"x"), object, Shape.__name__)
