# typy-test: FUN-05
# typy-expect: rejected E0414 at 13:10
class Shape:
    def area(self) -> float:
        return 0.0


class Square(Shape):
    def area(self) -> float:
        return 4.0


Square().area = Shape().area
