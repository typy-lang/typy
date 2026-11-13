# typy-test: CLS-14
# typy-expect: rejected E0201 at 15:16
class Shape:
    def describe(self) -> str:
        return "shape"


class Square(Shape):
    def describe(self) -> str:
        return "square"


class Tile(Square):
    def describe(self) -> str:
        return super(Square, self).describe()
