# typy-test: CLS-01
# typy-expect: rejected E0309 at 12:10
class Shape:
    def __init__(self, name: str) -> None:
        self.name = name


class Tagged(Shape):
    tag: str


tagged = Tagged("x")
