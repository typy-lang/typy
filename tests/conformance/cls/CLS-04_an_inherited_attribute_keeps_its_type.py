# typy-test: CLS-04
# typy-expect: rejected E0202 at 8:5
class Shape:
    name: str


class Circle(Shape):
    name: bytes
