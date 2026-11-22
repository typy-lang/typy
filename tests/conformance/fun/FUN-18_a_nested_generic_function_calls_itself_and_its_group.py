# typy-test: FUN-18
# typy-expect: accepted
from collections.abc import Callable


class Shape:
    def name(self) -> str:
        return "shape"


class Square(Shape):
    def name(self) -> str:
        return "square"


def outer(tag: str) -> list[str]:
    def label[S: Shape](item: S) -> str:
        return tag + item.name()

    def keep[T](value: T) -> Callable[[], T]:
        def give() -> T:
            return value

        return give

    def even[T](value: T, count: int) -> bool:
        if count == 0:
            return True
        return odd(value, count - 1)

    def odd[T](value: T, count: int) -> bool:
        if count == 0:
            return False
        return even(value, count - 1)

    def depth[T](value: T, count: int) -> int:
        if count == 0:
            return 0
        return 1 + depth(value, count - 1)

    held = keep(5)
    shown = [label(Square()) + label(Shape()), str(held()) + keep("w")()]
    shown.append(repr(held).split(" at ")[0])
    shown.append(str(even(1, 4)) + str(odd("x", 3)) + str(depth(2.5, 3)))
    shown.append(str(list(map(lambda each: keep(each)(), [7, 8]))))
    return shown


print(outer("#"))
