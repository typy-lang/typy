# typy-test: CLS-06, CLS-04, GEN-03
# typy-expect: accepted
class Shape:
    def __init__(self, name: str) -> None:
        self.name = name

    def area(self) -> float:
        return 0.0

    def describe(self, prefix: str = "a") -> str:
        return prefix + " " + self.name + " of " + str(self.area())

class Square(Shape):
    def __init__(self, side: float) -> None:
        Shape.__init__(self, "square")
        self.side = side

    def area(self) -> float:
        return self.side * self.side

    def describe(self, prefix: str = "the") -> str:
        return Shape.describe(self, prefix) + "!"

class Cube(Square):
    def __init__(self, side: float) -> None:
        Shape.__init__(self, "cube")
        self.side = side

    def area(self) -> float:
        return 6 * Square.area(self)

s = Square(2.0)
c = Cube(1.0)
print(s.describe(), c.describe(), Shape.area(s), Shape.describe(c, "one"))
print(Square.area(c), Shape.describe(s))
s2 = Square(3.0)
Square.__init__(s2, 5.0)
print(s2.side, s2.name)

class Box[T]:
    def __init__(self, item: T) -> None:
        self.item = item

    def get(self) -> T:
        return self.item

b = Box(4)
print(Box.get(b) + 1, Box[str].get(Box("x")))

class Pair[T]:
    def __init__(self, left: T, right: T) -> None:
        self.left = left
        self.right = right

    def first(self) -> T:
        return self.left

    def swapped(self) -> "Pair[T]":
        return Pair(Pair.first(self), self.right)

print(Pair.swapped(Pair(1, 2)).left, Pair.first(Pair("a", "b")))
