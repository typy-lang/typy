# typy-test: CLS-46
# typy-expect: rejected E0201 at 13:24
class Shape:
    @classmethod
    def kind(cls) -> str:
        return cls.__name__


class Square(Shape):
    def describe(self) -> str:
        # A class derived from Square defines `kind` again, so `cls` would be a class whose
        # copy of Shape.kind TyPy does not make.
        return super().kind()


class Tile(Square):
    @classmethod
    def kind(cls) -> str:
        return "tile"


print(Tile().describe())
