# typy-test: CLS-18
# typy-expect: accepted
class Shape:
    def __init__(self, size: int) -> None:
        self.size = size


class Square(Shape):
    pass


def describe(kind: type[Shape], shape: Shape) -> str:
    match shape:
        case kind(size=1):
            return "a unit of the kind"
        case kind():
            return "of the kind"
        case _:
            return "another shape"


print(describe(Square, Square(1)), describe(Square, Shape(2)))
