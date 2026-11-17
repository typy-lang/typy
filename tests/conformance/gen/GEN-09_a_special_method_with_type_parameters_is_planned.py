# typy-test: GEN-09
# typy-expect: rejected E0201 at 7:9
class Box[T]:
    def __init__(self, item: T) -> None:
        self.item = item

    def __eq__[U](self, other: U) -> bool:
        return False


print(Box(1) == Box(2))
