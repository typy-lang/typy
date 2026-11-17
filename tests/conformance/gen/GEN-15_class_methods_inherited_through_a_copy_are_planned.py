# typy-test: GEN-15
# typy-expect: rejected E0201 at 12:1
class Box[T]:
    def __init__(self, item: T) -> None:
        self.item = item

    @classmethod
    def make(cls, item: T) -> "Box[T]":
        return cls(item)


class IntBox(Box[int]):
    pass
