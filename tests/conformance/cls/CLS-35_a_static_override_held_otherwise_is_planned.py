# typy-test: CLS-35
# typy-expect: rejected E0201 at 10:9
class Shape:
    def scale(self, factor: int) -> int:
        return factor


class Square(Shape):
    @staticmethod
    def scale(factor: int | None) -> int:
        return 0
