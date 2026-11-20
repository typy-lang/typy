# typy-test: EXP-17
# typy-expect: accepted
class Shape:
    def name(self) -> str:
        return "shape"


class Square(Shape):
    def name(self) -> str:
        return "square"


def describe(shape: Shape | None) -> str:
    return "none" if shape is None else shape.name()


print(describe(*(Square(),)), describe(*(Shape(),)))
