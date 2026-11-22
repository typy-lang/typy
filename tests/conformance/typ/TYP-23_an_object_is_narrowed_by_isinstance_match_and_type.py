# typy-test: TYP-23
# typy-expect: accepted
class Point:
    def __init__(self, x: int, y: int) -> None:
        self.x = x
        self.y = y

    def __eq__(self, other: object) -> bool:
        if not isinstance(other, Point):
            return False
        return self.x == other.x and self.y == other.y


def total(values: list[object]) -> float:
    result = 0.0
    for value in values:
        if isinstance(value, float):
            result += value
        elif type(value) is int:
            result += value
    return result


def kind(value: object) -> str:
    match value:
        case bool():
            return "bool"
        case int():
            return "int"
        case str():
            return "str"
        case None:
            return "none"
        case Point():
            return "point"
        case _:
            return "other"


record: dict[str, object] = {"name": "ada", "age": 36, "point": Point(1, 2)}
print(Point(1, 2) == Point(1, 2), Point(1, 2) == 3, Point(1, 2) != "a")
print(total([1, 2.5, "x", True, None]))
samples: list[object] = [True, 1, "a", None, Point(0, 0), 2.5]
print([kind(v) for v in samples])
age = record["age"]
if type(age) is int:
    print(age * 2)
print(record["name"], "ada" in [record["name"]])
value: object = None
print(value is None, value == None, type(value).__name__, type(record["age"]).__name__)
if value is not None:
    print("not none")
items: set[object] = {1, "a", 1.0}
print(len(items), 1 in items, "b" in items)
