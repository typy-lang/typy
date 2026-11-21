# typy-test: FUN-09
# typy-expect: accepted
from collections.abc import Callable


class Shape:
    def sides(self) -> int:
        return 0


class Square(Shape):
    @staticmethod
    def sides() -> int:
        return 4


class Tile(Square):
    def sides(self) -> int:
        return 40


class Plain:
    @staticmethod
    def label(value: int) -> str:
        return f"plain {value}"


class Named(Plain):
    def label(self, value: int) -> str:
        return f"named {value}"


def count(shapes: list[Shape]) -> list[int]:
    counts: list[int] = []
    for shape in shapes:
        method: Callable[[], int] = shape.sides
        counts.append(method())
    return counts


def show(items: list[Plain]) -> list[str]:
    return [item.label(1) for item in items]


print(count([Shape(), Square(), Tile()]))
print(show([Plain(), Named()]))
square = Square()
held: Shape = square
print(held.sides == Square.sides, held.sides is Square.sides)
tile = Tile()
held = tile
print(held.sides == tile.sides, held.sides != Tile().sides)
named = Named()
plain: Plain = named
print(plain.label == named.label, Plain().label == Plain.label)
print(repr(plain.label)[:29], repr(Plain().label)[:27])
