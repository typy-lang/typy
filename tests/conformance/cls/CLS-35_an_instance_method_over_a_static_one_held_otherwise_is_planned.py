# typy-test: CLS-35
# typy-expect: rejected E0201 at 10:9
class Shape:
    @staticmethod
    def scale() -> int | None:
        return None


class Square(Shape):
    def scale(self) -> int:
        return 1
