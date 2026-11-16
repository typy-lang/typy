# typy-test: CLS-32
# typy-expect: rejected E0201 at 18:1
class Money:
    def __add__(self, other: "Money") -> "Money":
        return self


class Cash(Money):
    def __iadd__(self, other: Money) -> Money:
        return self


def pay(money: Money) -> Money:
    return money


wallet = pay(Cash())
wallet += Money()
