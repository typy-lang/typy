# typy-test: GEN-01, GEN-03
# typy-expect: accepted
class Box[T]:
    def __init__(self, value: T) -> None:
        self.value = value

    def get(self) -> T:
        return self.value

    def replaced(self, value: T) -> "Box[T]":
        return Box(value)


class Pair[K, V]:
    def __init__(self, key: K, value: V) -> None:
        self.key = key
        self.value = value

    def swapped(self) -> "Pair[V, K]":
        return Pair(self.value, self.key)


def unwrap[T](box: Box[T]) -> T:
    return box.get()


numbers = Box(1)
words = Box[str]("a")
print(numbers.get(), words.get(), unwrap(numbers), unwrap(words))
print(numbers.replaced(5).value, type(numbers).__name__, isinstance(words, Box))
print(type(numbers) is Box, type(numbers) is type(words))
pair = Pair("k", 2.5).swapped()
print(pair.key, pair.value)
boxes: list[Box[int]] = [Box(2), Box(3)]
print(boxes[0].get() + boxes[1].get())
