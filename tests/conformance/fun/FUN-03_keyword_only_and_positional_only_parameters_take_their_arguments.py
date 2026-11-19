# typy-test: FUN-03
# typy-expect: accepted
def scale(value: float, /, factor: float = 2.0, *, offset: float = 0.0, clamp: bool) -> float:
    result = value * factor + offset
    return min(result, 10.0) if clamp else result


def label(*, name: str, count: int = 1) -> str:
    return name * count


class Account:
    def __init__(self, owner: str, /, *, balance: int = 0) -> None:
        self.owner = owner
        self.balance = balance

    def deposit(self, amount: int, /, *, note: str = "") -> int:
        self.balance += amount
        return self.balance


print(scale(3.0, clamp=False), scale(3.0, 4.0, clamp=True), scale(1.0, factor=3.0, offset=0.5, clamp=False))
print(label(name="ab"), label(count=3, name="x"))
account = Account("ana", balance=5)
print(account.owner, account.deposit(10), account.deposit(1, note="tip"), account.balance)
