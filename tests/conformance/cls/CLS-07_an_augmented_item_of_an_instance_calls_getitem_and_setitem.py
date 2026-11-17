# typy-test: CLS-07
# typy-expect: accepted
class Money:
    def __init__(self, cents: int) -> None:
        self.cents = cents

    def __add__(self, other: "Money") -> "Money":
        print("add")
        return Money(self.cents + other.cents)


class Ledger:
    def __init__(self) -> None:
        self.entries = {"a": Money(1)}
        self.counts = [0, 0]

    def __getitem__(self, key: str) -> Money:
        print("get", key)
        return self.entries[key]

    def __setitem__(self, key: str, value: Money) -> None:
        print("set", key, value.cents)
        self.entries[key] = value


class Counts:
    def __init__(self) -> None:
        self.values = [0, 0]

    def __getitem__(self, index: int) -> int:
        print("get", index)
        return self.values[index]

    def __setitem__(self, index: int, value: int) -> None:
        print("set", index, value)
        self.values[index] = value


def ledger() -> Ledger:
    print("ledger")
    return book


def key() -> str:
    print("key")
    return "a"


book = Ledger()
ledger()[key()] += Money(5)
book["a"] += Money(3)
counts = Counts()
counts[1] += 4
counts[1] *= 3
counts[0] -= 1
print(book.entries["a"].cents, counts.values)
