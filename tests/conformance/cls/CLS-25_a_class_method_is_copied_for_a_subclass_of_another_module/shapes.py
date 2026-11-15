import math


class Shape:
    count = 0

    def __init__(self, name: str) -> None:
        self.name = name

    @classmethod
    def unit(cls) -> "Shape":
        Shape.count += 1
        return cls("unit " + cls.__name__ + " " + str(math.floor(2.5)))
