# typy-test: CLS-07
# typy-expect: accepted
import math


class Vector:
    def __init__(self, x: float, y: float) -> None:
        self.x = x
        self.y = y

    def __add__(self, other: "Vector") -> "Vector":
        return Vector(self.x + other.x, self.y + other.y)

    def __sub__(self, other: "Vector") -> "Vector":
        return Vector(self.x - other.x, self.y - other.y)

    def __mul__(self, factor: float) -> "Vector":
        return Vector(self.x * factor, self.y * factor)

    def __rmul__(self, factor: float) -> "Vector":
        return Vector(self.x * factor * 10.0, self.y * factor * 10.0)

    def __radd__(self, other: "Vector") -> "Vector":
        return Vector(other.x + self.x, other.y + self.y)

    def __neg__(self) -> "Vector":
        return Vector(-self.x, -self.y)

    def __abs__(self) -> float:
        return math.sqrt(self.x * self.x + self.y * self.y)

    def __repr__(self) -> str:
        return f"Vector({self.x}, {self.y})"


class Tagged(Vector):
    def __radd__(self, other: "Vector") -> "Vector":
        return Vector(0.0, 0.0)


a = Vector(1.0, 2.0)
b = Vector(3.0, 4.0)
print(a + b, b - a, a * 2.0, 3.0 * a, -a)
print(a + Tagged(5.0, 5.0), Tagged(1.0, 1.0) + a)
total = Vector(0.0, 0.0)
for each in [a, b]:
    total = total + each
print(total, abs(b))
