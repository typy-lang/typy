# typy-test: EXP-17
# typy-expect: accepted
class Shape:
    def name(self) -> str:
        return "shape"


class Square(Shape):
    def name(self) -> str:
        return "square"


def describe(shape: Shape) -> str:
    return shape.name()


print(describe(*(Square(),)))
