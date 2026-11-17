# typy-test: GEN-08, GEN-07, CST-06
# typy-expect: accepted
import tools
from tools import Box


class IntBox(tools.Box[int]):
    def get(self) -> int:
        return super().get() * 10


class StrBox(Box[str]):
    def shout(self) -> str:
        return self.get().upper()


class Keyed[K, V](Box[V]):
    def __init__(self, key: K, item: V) -> None:
        super().__init__(item)
        self.key = key

    def pair(self) -> tuple[K, V]:
        return (self.key, self.get())


def first(boxes: list[IntBox]) -> int:
    return min([box.get() for box in boxes])


a = IntBox(3)
b = IntBox(1)
print(first([a, b]), StrBox("x").shout(), IntBox.note(), a.note())
k = Keyed("k", 2.5)
print(k.pair(), Keyed(1, "v").pair(), isinstance(k, Box))
x: Box[int] = a
if isinstance(x, IntBox):
    print("int box", x.get())
match x:
    case IntBox(item=n):
        print("matched", n)
