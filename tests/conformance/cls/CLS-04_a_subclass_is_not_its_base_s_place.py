# typy-test: CLS-04
# typy-expect: rejected E0402 at 11:18
class Shape:
    pass


class Circle(Shape):
    pass


circle: Circle = Shape()
