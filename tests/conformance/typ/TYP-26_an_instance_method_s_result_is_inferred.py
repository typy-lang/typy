# typy-test: TYP-26
# typy-expect: accepted
# An instance method without a result annotation returns what its `return` statements
# give, also through the other methods it calls on its instance.
class Account:
    def __init__(self, owner: str, balance: int) -> None:
        self.owner = owner
        self.balance = balance

    def summary(self):
        return f"{self.label()}: {self.balance}"

    def label(self):
        return self.owner.title()

    def doubled(self):
        return Account(self.owner, self.balance * 2)

    @staticmethod
    def bank() -> str:
        return "TyPy Bank"

    def maybe_negative(self):
        if self.balance < 0:
            return self.balance
        return None


account = Account("ada", 10)
TEXT = account.summary()
print(TEXT.upper(), account.doubled().balance + 1, Account.bank(), account.maybe_negative())
