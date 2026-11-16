# typy-test: CLS-04
# typy-expect: accepted
class Shape:
    def scale(self, factor: int) -> int:
        return factor

    def name(self) -> str | None:
        return None


class Square(Shape):
    def scale(self, factor: int | None) -> int:
        print("Square.scale", factor)
        return 0 if factor is None else factor * 2

    def name(self) -> str:
        return "square"


class Big(Square):
    def scale(self, factor: int | None) -> int:
        print("Big.scale", factor)
        return 100 if factor is None else factor * 10


def run(shape: Shape) -> tuple[int, str | None]:
    return shape.scale(3), shape.name()


for each in [Shape(), Square(), Big()]:
    print(run(each))
square: Square = Big()
print(square.scale(None), square.scale(4), square.name())
