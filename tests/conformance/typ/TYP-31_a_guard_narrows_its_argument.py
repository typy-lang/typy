# typy-test: TYP-31
# typy-python: 3.13
# typy-expect: accepted
from typing import TypeGuard, TypeIs


class Shape:
    pass


class Circle(Shape):
    def label(self) -> str:
        return "circle"


def is_text(value: object) -> TypeIs[str]:
    return isinstance(value, str)


def is_circle(shape: Shape) -> TypeGuard[Circle]:
    return isinstance(shape, Circle)


def describe(value: int | str) -> str:
    if is_text(value):
        return value.upper()
    return str(value + 1)


def name(shape: Shape) -> str:
    if is_circle(shape):
        return shape.label()
    return "shape"


print(describe("a"), describe(2), name(Circle()), name(Shape()), is_text(3))


def is_small(value: object) -> TypeIs[int | None]:
    return value is None or isinstance(value, int)


def shown(value: int | str | None) -> str:
    if is_small(value):
        return str(value)
    return value.upper()


print(shown("a"), shown(None), shown(2))
