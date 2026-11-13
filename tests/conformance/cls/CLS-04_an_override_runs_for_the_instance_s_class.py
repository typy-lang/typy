# typy-test: CLS-04
# typy-expect: accepted
class Shape:
    def __init__(self, name: str) -> None:
        self.name = name

    def area(self) -> float:
        return 0.0

    def describe(self) -> str:
        return self.name + " of area " + str(self.area())

    def scaled(self, factor: float = 2.0) -> "Shape":
        return Shape(self.name)

    def __str__(self) -> str:
        return "Shape " + self.name


class Square(Shape):
    def __init__(self, side: float) -> None:
        self.name = "square"
        self.side = side

    def area(self) -> float:
        return self.side * self.side

    def scaled(self, factor: float = 2.0) -> "Square":
        return Square(self.side * factor)

    def __str__(self) -> str:
        return "Square " + str(self.side)


class Checked(Square):
    def area(self) -> float:
        if self.side < 0:
            raise ValueError("negative side")
        return self.side * self.side


class Group(Shape):
    def __init__(self, parts: list[Shape]) -> None:
        self.name = "group"
        self.parts = parts

    def area(self) -> float:
        total = 0.0
        for part in self.parts:
            total += part.area()
        return total


class Counter:
    def __init__(self, limit: int) -> None:
        self.limit = limit
        self.current = 0

    def __iter__(self) -> "Counter":
        return self

    def __next__(self) -> int:
        if self.current >= self.limit:
            raise StopIteration
        self.current += 1
        return self.current


class Evens(Counter):
    def __next__(self) -> int:
        if self.current >= self.limit:
            raise StopIteration
        self.current += 2
        return self.current


shapes: list[Shape] = [Shape("blob"), Square(2.0), Checked(3.0)]
shapes.append(Group([Square(1.0), Group([Checked(2.0)])]))
for shape in shapes:
    print(shape.describe(), shape.scaled().area(), shape, type(shape).__name__)
bad: Shape = Checked(-1.0)
try:
    print(bad.describe())
except ValueError as error:
    print("error:", error)
counters: list[Counter] = [Counter(3), Evens(6)]
for counter in counters:
    print(list(counter))
print(next(iter(Evens(4))))
