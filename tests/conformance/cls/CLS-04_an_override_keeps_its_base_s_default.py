# typy-test: CLS-04
# typy-expect: rejected E0201 at 9:9
class Shape:
    def scaled(self, factor: float = 2.0) -> float:
        return factor


class Square(Shape):
    def scaled(self, factor: float = 3.0) -> float:
        return factor
