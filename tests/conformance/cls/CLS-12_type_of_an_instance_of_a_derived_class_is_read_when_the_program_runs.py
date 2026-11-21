# typy-test: CLS-12
# typy-expect: accepted
class Shape:
    def name(self) -> str:
        return "shape"


class Circle(Shape):
    def name(self) -> str:
        return "circle"


class Square(Shape):
    pass


def describe(shape: Shape) -> str:
    kind = type(shape)
    return kind.__name__ + " " + str(kind is Circle) + " " + str(kind == Shape)


print(describe(Circle()), describe(Square()), describe(Shape()))
print(type(Circle()), [type(each).__name__ for each in [Shape(), Circle()]])
