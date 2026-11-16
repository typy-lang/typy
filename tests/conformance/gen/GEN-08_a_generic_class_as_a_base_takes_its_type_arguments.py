# typy-test: GEN-08
# typy-expect: rejected E0408 at 8:11
class Box[T]:
    def __init__(self, item: T) -> None:
        self.item = item


class Sub(Box):
    pass
