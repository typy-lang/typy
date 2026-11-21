# typy-test: CLS-39
# typy-expect: rejected E0201 at 12:12
class Shape:
    pass


class Square(Shape):
    pass


def make(kind: type[Shape]) -> Shape:
    return kind()


print(make(Square))
