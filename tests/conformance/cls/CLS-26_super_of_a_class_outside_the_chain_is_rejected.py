# typy-test: CLS-26
# typy-expect: rejected E0404 at 14:22
class Shape:
    def describe(self) -> str:
        return "shape"


class Other:
    pass


class Square(Shape):
    def describe(self) -> str:
        return super(Other, self).describe()
