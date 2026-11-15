class Shape:
    def __init__(self, name: str) -> None:
        self.name = name

    def describe(self) -> str:
        return "a shape named " + self.name


class Problem(ValueError):
    pass
