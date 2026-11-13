# typy-test: CLS-07
# typy-expect: accepted
class Money:
    def __init__(self, cents: int) -> None:
        self.cents = cents

    def __add__(self, other: "Money") -> "Money":
        return Money(self.cents + other.cents)

    def __eq__(self, other: "Money") -> bool:
        return self.cents == other.cents

    def __hash__(self) -> int:
        return self.cents

    def __repr__(self) -> str:
        return f"Money({self.cents})"


class Wallet:
    def __init__(self) -> None:
        self.total = Money(0)

    def add(self, amount: Money) -> None:
        self.total += amount


total = Money(0)
wallet = Wallet()
for cents in [100, 250, 5]:
    total += Money(cents)
    wallet.add(Money(cents))
matches = 0
for value in [Money(355), Money(1)]:
    if value == total:
        matches += 1
print(total, wallet.total, matches, total == wallet.total)
