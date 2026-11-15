# typy-test: CLS-06
# typy-expect: rejected E0201 at 10:7
class Shape:
    @classmethod
    def unit(cls) -> "Shape":
        return cls()


shape = Shape()
shape.unit()
