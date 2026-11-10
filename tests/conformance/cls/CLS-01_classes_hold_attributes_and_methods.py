# typy-test: CLS-01
# typy-expect: accepted
class Account:
    """A bank account with a running balance."""

    owner: str

    def __init__(self, owner: str, balance: int = 0) -> None:
        self.owner = owner
        self.balance = balance
        self.history: list[int] = []
        self.note: str | None = None

    def deposit(self, amount: int) -> None:
        self.balance += amount
        self.history.append(amount)

    def withdraw(self, amount: int) -> bool:
        if amount > self.balance:
            self.note = "refused " + str(amount)
            return False
        self.deposit(-amount)
        return True


class Ledger:
    def __init__(self) -> None:
        self.accounts: dict[str, Account] = {}

    def open(self, owner: str) -> Account:
        account = Account(owner)
        self.accounts[owner] = account
        return account

    def total(self) -> int:
        return sum([account.balance for account in self.accounts.values()])


ledger = Ledger()
alice = ledger.open("alice")
alice.deposit(100)
print(alice.withdraw(30), alice.withdraw(500), alice.note)
bob = ledger.open("bob")
bob.deposit(7)
bob.balance *= 3
print(alice.balance, alice.history, bob.balance, ledger.total())
print(ledger.accounts["bob"] is bob, alice == bob, alice != bob, bool(alice))
