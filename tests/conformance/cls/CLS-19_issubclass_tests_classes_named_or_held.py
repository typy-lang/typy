# typy-test: CLS-19
# typy-expect: accepted
class Shape:
    pass


class Square(Shape):
    pass


class Circle(Shape):
    pass


kind = type(Square())
print(issubclass(Square, Shape), issubclass(Shape, Square), issubclass(Shape, Shape))
print(issubclass(bool, int), issubclass(int, float), issubclass(kind, (Circle, Shape)))
print(issubclass(ValueError, (KeyError, ArithmeticError)), issubclass(ZeroDivisionError, ArithmeticError))
print(issubclass(kind, Circle), issubclass(type(True), int), issubclass(Square, object))
