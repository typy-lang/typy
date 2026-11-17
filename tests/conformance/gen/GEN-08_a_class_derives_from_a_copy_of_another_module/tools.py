class Box[T]:
    def __init__(self, item: T) -> None:
        self.item = item

    def get(self) -> T:
        return self.item

    @staticmethod
    def note() -> str:
        return "note"
