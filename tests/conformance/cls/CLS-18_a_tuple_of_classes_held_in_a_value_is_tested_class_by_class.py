# typy-test: CLS-18, CLS-19
# typy-expect: accepted
class Shape:
    pass


class Square(Shape):
    pass


class Circle(Shape):
    pass


numbers = (int, float)
shapes = (Square, Circle)
nothing = ()


def kind(value: int | str | float) -> str:
    if isinstance(value, numbers):
        return "number"
    return "text"


def pick() -> tuple[type[Square], type[Circle]]:
    print("picked")
    return (Square, Circle)


print(kind(1), kind("a"), kind(2.5), isinstance(Square(), shapes), isinstance(Shape(), shapes))
print(isinstance(3, nothing), isinstance(Circle(), pick()), isinstance(True, numbers))
print(issubclass(Square, shapes), issubclass(Shape, shapes), issubclass(bool, numbers))
print(issubclass(Circle, pick()), issubclass(int, (str, numbers[0])))
