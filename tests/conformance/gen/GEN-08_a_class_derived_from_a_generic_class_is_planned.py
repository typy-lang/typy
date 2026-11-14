# typy-test: GEN-08
# typy-expect: rejected E0201 at 8:1
class Box[T]:
    def __init__(self, value: T) -> None:
        self.value = value


class Numbers(Box[int]):
    pass
