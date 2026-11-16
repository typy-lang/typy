# typy-test: GEN-11
# typy-expect: rejected E0201 at 4:11
class Box[T]:
    last: T | None = None

    def __init__(self, item: T) -> None:
        self.item = item


print(Box(1).last)
