# typy-test: GEN-08
# typy-expect: rejected E0201 at 3:1
class Counter[T]:
    made = 0

    def __init__(self, value: T) -> None:
        self.value = value
