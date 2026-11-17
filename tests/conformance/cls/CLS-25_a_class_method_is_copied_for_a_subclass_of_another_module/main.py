# typy-test: CLS-25
# typy-expect: accepted
from shapes import Shape


class Square(Shape):
    pass


class Tile(Square):
    def __init__(self, name: str) -> None:
        super().__init__(name + "!")


print(Square.unit().name, Tile.unit().name, type(Tile.unit()).__name__, Shape.count)
