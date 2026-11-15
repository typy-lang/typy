# typy-test: CLS-04
# typy-expect: rejected E0404 at 12:9
class Shape:
    def __init__(self, name: str) -> None:
        self.name = name

    def resize(self, factor: float) -> None:
        self.name = self.name + "!"


class Square(Shape):
    def resize(self, factor: int) -> None:
        self.name = self.name + "?"
