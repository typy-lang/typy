# typy-test: CLS-25
# typy-expect: accepted
class Shape:
    def __init__(self, name: str) -> None:
        self.name = name

    @classmethod
    def unit(cls, suffix: str = "") -> "Shape":
        return cls(cls.__name__ + suffix)

    def again(self) -> "Shape":
        return self.unit("!")


class Square(Shape):
    pass


class Plain(Shape):
    def unit(self, suffix: str = "") -> "Shape":
        return Shape("plain " + suffix)


def make(shape: Shape) -> str:
    return shape.unit("?").name


shapes: list[Shape] = [Shape("a"), Square("b"), Plain("c")]
print([make(each) for each in shapes], Square("x").again().name, type(Square("y").unit()).__name__)


