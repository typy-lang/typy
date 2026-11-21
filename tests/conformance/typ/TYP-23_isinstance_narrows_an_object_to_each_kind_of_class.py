# typy-test: TYP-23
# typy-expect: accepted
class Box[T]:
    def __init__(self, item: T) -> None:
        self.item = item


def describe(value: object) -> str:
    if isinstance(value, float):
        return str(value + 0.5)
    if isinstance(value, bool):
        return str(not value)
    if isinstance(value, bytes):
        return str(len(value))
    if isinstance(value, type(None)):
        return "none"
    if isinstance(value, type):
        return value.__name__
    if isinstance(value, range):
        return str(len(value))
    if isinstance(value, slice):
        return str(value.start)
    if isinstance(value, ValueError):
        return str(value)
    if isinstance(value, Box):
        return "box"
    if isinstance(value, object):
        return "object"
    return "other"


print(describe(1.5), describe(True), describe(b"ab"), describe(None), describe(int))
print(describe(range(3)), describe(slice(1, 2)), describe(ValueError("x")), describe(Box(1)))
