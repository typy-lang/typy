# typy-test: CLS-29
# typy-expect: rejected E0201 at 10:16
class Shape:
    def describe(self) -> str:
        return "shape"


class Square(Shape):
    def other(self, shape: "Square") -> str:
        return super(Square, shape).describe()
