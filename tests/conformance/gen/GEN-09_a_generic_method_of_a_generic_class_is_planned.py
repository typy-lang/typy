# typy-test: GEN-09
# typy-expect: rejected E0201 at 7:5
class Box[T]:
    def __init__(self, value: T) -> None:
        self.value = value

    def paired[U](self, other: U) -> tuple[T, U]:
        return (self.value, other)
