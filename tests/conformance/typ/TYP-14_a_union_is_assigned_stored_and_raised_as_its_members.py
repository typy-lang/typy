# typy-test: TYP-14
# typy-expect: accepted
class Shape:
    def __init__(self, name: str) -> None:
        self.name = name

    def __repr__(self) -> str:
        return "Shape(" + self.name + ")"


class Circle(Shape):
    pass


class Square(Shape):
    def __repr__(self) -> str:
        return "Square!"


def rename(shape: Circle | Square, name: str) -> None:
    shape.name = name


value: str | bytes = "text"
print(value)
value = b"raw"
print(value)
shapes: dict[str, Circle | Square] = {"c": Circle("c"), "s": Square("s")}
rename(shapes["c"], "round")
print(shapes, shapes["c"].name, isinstance(shapes["s"], Shape))
problem: ValueError | KeyError = KeyError("k")
print(repr(problem), isinstance(problem, KeyError))
try:
    raise ValueError("bad")
except (ValueError, KeyError) as error:
    problem = error
print(problem, type(problem).__name__)
pairs: list[tuple[str, str | list[str]]] = [("a", "x"), ("b", ["y"])]
for key, item in pairs:
    if isinstance(item, list):
        print(key, len(item), item)
    else:
        print(key, item.upper())
