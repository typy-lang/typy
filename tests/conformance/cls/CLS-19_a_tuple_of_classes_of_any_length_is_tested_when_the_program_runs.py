# typy-test: CLS-19
# typy-expect: accepted
class Shape:
    pass


class Square(Shape):
    pass


kinds: tuple[type[Square], ...] = (Square, Square)
none: tuple[type[Square], ...] = ()


def describe(shape: Shape) -> str:
    if isinstance(shape, kinds):
        return "known"
    return "other"


print(describe(Square()), describe(Shape()), isinstance(Square(), none), isinstance(3, kinds))
print(issubclass(Square, kinds), issubclass(Shape, kinds), issubclass(Square, none))
