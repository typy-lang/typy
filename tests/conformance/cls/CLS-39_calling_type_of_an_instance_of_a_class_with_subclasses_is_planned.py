# typy-test: CLS-39
# typy-expect: rejected E0201 at 15:12
class Shape:
    def name(self) -> str:
        return "shape"


class Circle(Shape):
    def name(self) -> str:
        return "circle"


def copy(shape: Shape) -> Shape:
    kind = type(shape)
    return kind()


print(copy(Circle()).name())
