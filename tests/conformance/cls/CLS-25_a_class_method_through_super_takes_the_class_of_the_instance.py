# typy-test: CLS-25, CLS-14
# typy-expect: accepted
class Shape:
    def __init__(self, label: str) -> None:
        self.label = label

    @classmethod
    def kind(cls, sides: int) -> str:
        return f"{cls.__name__} of {sides}"


class Square(Shape):
    def through_super(self) -> str:
        return super().kind(4)

    def named_super(self) -> str:
        return super(Square, self).kind(4)

    def bound_first(self) -> str:
        # super() binds the instance before the argument rebinds `self`.
        return super().kind(len((self := Tile("other")).label))


class Tile(Square):
    pass


for shape in [Square("square"), Tile("tile")]:
    print(shape.through_super(), shape.named_super(), shape.bound_first())
