# typy-test: FUN-09
# typy-expect: rejected E0201 at 9:14
class Shape:
    @classmethod
    def unit(cls) -> "Shape":
        return cls()


make = Shape.unit
