# typy-test: TYP-16
# typy-expect: accepted
items: list[int | None] = [None, None]
a = items[0] = 5
print(a, items)
flags: dict[str, int | str] = {}
b = flags["x"] = 7
print(b, flags)
x: int | None
y: int
x = y = 3
print(x, y)
class Box:
    def __init__(self) -> None:
        self.value: float | None = None
box = Box()
c = box.value = 2.5
print(c, box.value)
values: list[float] = [0.0]
values[0] = d = 4.0
print(values, d)
