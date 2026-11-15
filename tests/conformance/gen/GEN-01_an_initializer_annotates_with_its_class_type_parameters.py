# typy-test: GEN-01
# typy-expect: accepted
class Box[T]:
    def __init__(self, value: T) -> None:
        self.value: T = value
        held: list[T] = [value]
        self.items = held


numbers: Box[int] = Box(1)
words = Box("a")
print(numbers.value, numbers.items, words.value, words.items)
