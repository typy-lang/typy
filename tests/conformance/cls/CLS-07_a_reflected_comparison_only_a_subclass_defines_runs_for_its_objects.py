# typy-test: CLS-07
# typy-expect: accepted
class Base:
    def __init__(self, n: int) -> None:
        self.n = n

    def __lt__(self, other: "Base") -> bool:
        print("Base.__lt__")
        return self.n < other.n


class Picky(Base):
    def __gt__(self, other: "Base") -> bool:
        print("Picky.__gt__")
        return True


def less(a: Base, b: Base) -> bool:
    return a < b


print(less(Base(1), Base(2)), less(Base(5), Picky(1)), less(Picky(5), Base(1)))
