# typy-test: GEN-07, GEN-03
# typy-expect: accepted
import tools
from tools import Box, Pair, doubled


class Point:
    def __init__(self, x: int) -> None:
        self.x = x

    def name(self) -> str:
        return "point " + str(self.x)


class Holder:
    def __init__(self) -> None:
        self.box: Box[float] = Box(1.5)


b = tools.Box(3)
print(type(b), type(b).__name__)
print(isinstance(b, Box))
print(doubled(Point(2))[1].x)
print(tools.greet(Point(4)))
print(tools.loud(Point(5)).x)
print(tools.grown(3), tools.grown(2.5))
print(Pair("a", 1), Pair(1.5, [2]) == Pair(1.5, [2]))
print(Holder().box.get())
print(tools.Maker().make("m"))
