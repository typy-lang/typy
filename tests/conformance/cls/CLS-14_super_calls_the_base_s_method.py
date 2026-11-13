# typy-test: CLS-14, CLS-04
# typy-expect: accepted
class Shape:
    def __init__(self, name: str) -> None:
        self.name = name

    def describe(self) -> str:
        return "shape " + self.name

    def area(self) -> float:
        return 0.0


class Square(Shape):
    def __init__(self, side: float) -> None:
        super().__init__("square")
        self.side = side

    def describe(self) -> str:
        return super().describe() + " with side " + str(self.side)

    def area(self) -> float:
        return self.side * self.side + super(Square, self).area()


class Tile(Square):
    def describe(self) -> str:
        return "tile, " + super().describe()


class Labelled(Shape):
    def __init__(self, label: str) -> None:
        self.label = label
        super().__init__("labelled")

    def describe(self) -> str:
        return "label " + self.label


class Announced(Shape):
    def __init__(self, name: str) -> None:
        super().__init__(name)
        print("made", self.describe())


class Plain:
    def __init__(self) -> None:
        super().__init__()
        self.count = 1


shapes: list[Shape] = [Shape("blob"), Square(2.0), Tile(1.5), Labelled("x")]
shapes.append(Announced("loud"))
for shape in shapes:
    print(shape.describe(), shape.area())
print(Plain().count)
