# typy-test: CLS-34
# typy-expect: rejected E0201 at 9:9
class Shape:
    def scale(self, factor: int = 1) -> int:
        return factor


class Square(Shape):
    def scale(self, factor: int | None = None) -> int:
        return 0 if factor is None else factor
