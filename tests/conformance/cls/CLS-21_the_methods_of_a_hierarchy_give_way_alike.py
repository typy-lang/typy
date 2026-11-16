# typy-test: CLS-21
# typy-expect: accepted
class Shape:
    def __init__(self, size: int) -> None:
        self.size = size

    def __eq__(self, other: "Shape") -> bool:
        print("Shape.__eq__")
        return self.size == other.size

    def __ne__(self, other: "Shape") -> bool:
        print("Shape.__ne__")
        return NotImplemented

    def __add__(self, other: "Shape") -> "Shape":
        return Shape(self.size + other.size)

    def __radd__(self, other: "Shape") -> "Shape":
        print("Shape.__radd__")
        return Shape(other.size * 10 + self.size)


class Square(Shape):
    def __eq__(self, other: "Shape") -> bool:
        print("Square.__eq__")
        if not isinstance(other, Square):
            return NotImplemented
        return self.size == other.size

    def __radd__(self, other: "Shape") -> "Shape":
        print("Square.__radd__")
        if other.size > 5:
            return NotImplemented
        return Shape(other.size * 100 + self.size)


def same(a: Shape, b: Shape) -> bool:
    return a == b


s = Shape(1)
q = Square(1)
print(same(s, q), same(q, s), same(q, Square(1)), same(s, Shape(1)))
print(s != q, q != s, s != s)
print((s + q).size, (Shape(9) + q).size, (q + s).size)
print(q in [s], s in [Shape(5), q], [Shape(2), q].index(Square(1)))
x = NotImplemented
values = [x, NotImplemented]
print(values, x is NotImplemented, values[0] is values[1])
