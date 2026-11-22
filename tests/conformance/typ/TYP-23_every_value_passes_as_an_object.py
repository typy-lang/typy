# typy-test: TYP-23
# typy-expect: accepted
class Box:
    def __init__(self, value: object) -> None:
        self.value: object = value

    def describe(self) -> str:
        return f"Box({self.value!r})"


def make(flag: bool) -> object:
    if flag:
        return 1
    return "one"


items: list[object] = []
items.append(1)
items.append("a")
items.append(None)
items.append((2, 3.5))
items.extend([Box(4).value, make(False)])
print(items, len(items), items[0] == 1, items[1] != "b")
pair: tuple[int, object] = (1, "x")
print(pair, pair[1] == "x")
x: object = 3
x = "now a str"
print(x, f"{x}", str(x), repr(x), bool(x), make(True) == 1)
maybe: object | None = None
print(maybe, maybe is None)
if x:
    print("truthy")
for each in items:
    if each is None:
        print("found None")
print(Box([1, 2]).describe(), Box(2.5).describe())
nested: dict[str, list[object]] = {"a": [1, "b"]}
print(nested)
