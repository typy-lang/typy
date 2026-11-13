# typy-test: MOD-06, MOD-03
# typy-expect: accepted
# typy-python: 3.15
lazy import heavy
lazy from shapes import Point, area, UNIT
lazy import unused

if UNIT > 0:
    lazy import tools.text as text


class Box:
    def __init__(self) -> None:
        self.corner = Point(2)


def total() -> int:
    return heavy.VALUE + UNIT


UNIT = 5
print("before any use", UNIT)
print(area(Point(3)))
print(Box().corner.x)
print(total())
print(text.LOUD, type(heavy).__name__)
