# typy-test: CLS-20
# typy-expect: accepted
class Vector:
    def __init__(self, x: int) -> None:
        self.x = x

    def __add__(self, other: "Vector") -> "Vector":
        print("add")
        return Vector(self.x + other.x)

    def __iadd__(self, other: "Vector") -> "Vector":
        print("iadd")
        self.x += other.x
        return self

    def __mul__(self, other: int) -> "Vector":
        return Vector(self.x * other)


class Box:
    def __init__(self) -> None:
        self.items = [Vector(1), Vector(2)]
        self.inner = Vector(5)


def get(box: Box) -> Box:
    print("get")
    return box


v = Vector(1)
w = v
v += Vector(2)
print(v.x, w.x, v is w)
v *= 3
print(v.x, w.x, v is w)
box = Box()
box.items[1] += Vector(10)
get(box).inner += Vector(1)
get(box).items[0] *= 4
print([item.x for item in box.items], box.inner.x)
