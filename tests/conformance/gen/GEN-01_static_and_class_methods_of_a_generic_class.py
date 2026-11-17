# typy-test: GEN-01, GEN-03, CLS-06
# typy-expect: accepted
class Box[T]:
    def __init__(self, item: T) -> None:
        self.item = item

    @staticmethod
    def count() -> int:
        return 2

    @staticmethod
    def wrap(item: T) -> list[T]:
        return [item]

    @classmethod
    def make(cls, item: T) -> "Box[T]":
        return cls(item)

    @classmethod
    def name(cls) -> str:
        return "box"


b = Box(1)
print(Box.count(), b.count(), Box[str].count())
print(Box.wrap("a"), b.wrap(2), Box[float].wrap(1.5))
print(Box.make(3).item, Box[str].make("s").item, Box[int].name())
