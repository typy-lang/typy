# typy-test: CLS-04
# typy-expect: rejected E0309 at 14:9
class Shape:
    def __init__(self, name: str) -> None:
        self.name = name
        print(self.describe())

    def describe(self) -> str:
        return self.name


class Labelled(Shape):
    def __init__(self, label: str) -> None:
        super().__init__("labelled")
        self.label = label

    def describe(self) -> str:
        return self.label
