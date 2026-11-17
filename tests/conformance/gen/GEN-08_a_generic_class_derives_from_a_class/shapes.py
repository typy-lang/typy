class Shape:
    def __init__(self, name: str) -> None:
        self.name = name

    def area(self) -> float:
        return 0.0

    def report(self) -> str:
        return self.name + " " + str(self.area())
