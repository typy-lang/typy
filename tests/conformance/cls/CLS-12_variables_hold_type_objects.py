# typy-test: CLS-12
# typy-expect: accepted
class Point:
    pass


def kind_of(value: int) -> str:
    found = type(value)
    names = [found, str]
    return found.__name__ + " " + str(len(names))


kind = set
other: type = Point
if kind is set:
    other = int
print(kind, other, kind_of(3), type(Point()) is Point)
