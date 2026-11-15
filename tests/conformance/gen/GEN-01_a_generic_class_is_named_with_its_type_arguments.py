# typy-test: GEN-01
# typy-expect: rejected E0408 at 8:7
class Box[T]:
    def __init__(self, value: T) -> None:
        self.value = value


held: Box = Box(1)
