# typy-test: GEN-09, GEN-01, GEN-03
# typy-expect: accepted
class Box[T]:
    def __init__(self, value: T) -> None:
        self.value = value

    def paired[U](self, other: U) -> tuple[T, U]:
        return (self.value, other)

    def mapped[U](self, items: list[U]) -> list[tuple[T, U]]:
        return [self.paired(item) for item in items]

    @staticmethod
    def both[U](first: U, second: U) -> list[U]:
        return [first, second]


class IntBox(Box[int]):
    def doubled(self) -> int:
        return self.value * 2


b = Box(1)
s = Box("s")
print(b.paired("x"), b.paired(2.5), s.paired(3), s.mapped([1, 2]))
print(Box.both(1, 2), b.both("a", "b"), Box[str].both(1.5, 2.5))
i = IntBox(4)
print(i.paired("p"), i.mapped(["q"]), i.doubled())
