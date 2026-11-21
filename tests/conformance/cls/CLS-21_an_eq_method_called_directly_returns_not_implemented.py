# typy-test: CLS-21
# typy-expect: accepted
class Money:
    def __init__(self, amount: int) -> None:
        self.amount = amount

    def __eq__(self, other: "Money | int") -> bool:
        if isinstance(other, int):
            return NotImplemented
        return self.amount == other.amount


result = Money(1).__eq__(2)
print(NotImplemented is result, result is NotImplemented)
