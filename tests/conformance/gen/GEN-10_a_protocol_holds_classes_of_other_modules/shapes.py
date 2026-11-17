class Circle:
    def __init__(self, radius: float) -> None:
        self.radius = radius

    def area(self) -> float:
        return 3.0 * self.radius * self.radius

    def __str__(self) -> str:
        return "Circle(" + str(self.radius) + ")"

    def __eq__(self, other: "Circle") -> bool:
        return self.radius == other.radius


class Big(Circle):
    def area(self) -> float:
        return 100.0


class Box[T]:
    def __init__(self, item: T) -> None:
        self.item = item

    def area(self) -> float:
        return 1.5
