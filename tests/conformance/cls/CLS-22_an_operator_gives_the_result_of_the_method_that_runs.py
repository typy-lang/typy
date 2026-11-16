# typy-test: CLS-22
# typy-expect: accepted
class Vector:
    def __init__(self, x: int) -> None:
        self.x = x

    def __add__(self, other: "Vector") -> "Vector":
        return Vector(self.x + other.x)

    def __radd__(self, other: "Vector") -> int:
        return other.x * 100 + self.x


class Scaled(Vector):
    def __radd__(self, other: "Vector") -> int:
        return -1


class Gauge:
    def __init__(self, x: int) -> None:
        self.x = x

    def __mul__(self, other: "Gauge") -> "Gauge":
        if other.x < 0:
            return NotImplemented
        return Gauge(self.x * other.x)

    def __rmul__(self, other: "Gauge") -> str:
        return "reflected"


class Big(Gauge):
    pass


def show(value: "Vector | int") -> str:
    if isinstance(value, Vector):
        return f"Vector({value.x})"
    return f"int {value}"


print(show(Vector(1) + Vector(2)), show(Vector(1) + Scaled(2)), show(Scaled(1) + Vector(2)))
for result in [Gauge(2) * Gauge(3), Gauge(2) * Big(-1), Gauge(2) * Big(3)]:
    if isinstance(result, str):
        print(result)
    else:
        print(result.x)
