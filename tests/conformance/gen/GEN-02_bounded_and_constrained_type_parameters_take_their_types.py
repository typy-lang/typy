# typy-test: GEN-02
# typy-expect: accepted
class Shape:
    def area(self) -> float:
        return 0.0


class Square(Shape):
    def __init__(self, side: float) -> None:
        self.side = side

    def area(self) -> float:
        return self.side * self.side


def largest[T: Shape](shapes: list[T]) -> T:
    best = shapes[0]
    for shape in shapes:
        if shape.area() > best.area():
            best = shape
    return best


def joined[S: (str, bytes)](first: S, second: S) -> S:
    return first + second


print(largest([Square(1.0), Square(3.0)]).area())
print(joined("a", "b"), joined(b"c", b"d"))
