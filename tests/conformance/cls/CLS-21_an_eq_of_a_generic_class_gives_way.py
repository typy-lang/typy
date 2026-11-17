# typy-test: CLS-21
# typy-expect: accepted
class Pair[T]:
    def __init__(self, first: T, second: T) -> None:
        self.first = first
        self.second = second

    def __eq__(self, other: "Pair[T]") -> bool:
        if self.first != other.first:
            return NotImplemented
        return self.second == other.second


a = Pair(1, 2)
b = Pair(1, 2)
c = Pair(3, 2)
print(a == b, a == c, a != c, a in [c, b], Pair("x", "y") == Pair("x", "y"))
