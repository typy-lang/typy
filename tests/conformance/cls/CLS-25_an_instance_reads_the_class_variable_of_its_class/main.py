# typy-test: CLS-25
# typy-expect: accepted
from shapes import Shape, show


class Square(Shape):
    pass


square = Square()
print(show(square), show(Shape()))
Square.sides = 4
print(show(square), show(Shape()), Square.sides)
