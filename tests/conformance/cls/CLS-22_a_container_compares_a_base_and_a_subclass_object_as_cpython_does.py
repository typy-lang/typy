# typy-test: CLS-22
# typy-expect: accepted
class Base:
    def __init__(self, n: int) -> None:
        self.n = n

    def __lt__(self, other: "Base") -> bool:
        print("lt", self.n, other.n)
        return self.n < other.n

    def __gt__(self, other: "Base") -> bool:
        print("gt", self.n, other.n)
        return self.n > other.n

    def __eq__(self, other: "Base") -> bool:
        print("eq", self.n, other.n)
        return self.n == other.n

    def __hash__(self) -> int:
        return self.n


class Sub(Base):
    pass


items = [Base(3), Sub(1), Base(2), Sub(4)]
print([item.n for item in sorted([Base(3), Sub(1)])])
print(min(items).n, max(items).n)
print(Sub(2) in [Base(1), Base(2)], [Base(5), Sub(5)].index(Sub(5)))
same = [Base(2), Base(1)]
print([item.n for item in sorted(same)])
