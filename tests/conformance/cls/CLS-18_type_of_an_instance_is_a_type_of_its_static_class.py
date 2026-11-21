# typy-test: CLS-18
# typy-expect: accepted
class Shape:
    kind = "shape"

    @classmethod
    def make(cls) -> int:
        return 1


class Circle(Shape):
    kind = "circle"


shape: Shape = Circle()


def build(kind: type[Shape]) -> str:
    return kind.__name__


print(type(shape), type(shape).__name__, type(shape) is Circle, build(type(shape)))
print(isinstance(Shape(), type(shape)), issubclass(type(shape), Shape))
