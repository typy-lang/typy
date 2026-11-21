# typy-test: FUN-17
# typy-expect: rejected E0201 at 12:14
class Box:
    def __init__(self) -> None:
        self.n = 1

    @property
    def v(self) -> int:
        return self.n


getter = Box.v
print(getter(Box()))
