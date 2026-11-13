# typy-test: CLS-06
# typy-expect: rejected E0414 at 13:1
class Circle:
    def __init__(self, radius: float) -> None:
        self.radius = radius

    @property
    def area(self) -> float:
        return 3.14159 * self.radius * self.radius


circle = Circle(1.0)
circle.area = 2.0
