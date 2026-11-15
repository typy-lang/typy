# typy-test: GEN-02
# typy-expect: rejected E0408 at 12:14
class Shape:
    pass


class Holder[S: Shape]:
    def __init__(self, shape: S) -> None:
        self.shape = shape


held: Holder[int] = Holder(Shape())
