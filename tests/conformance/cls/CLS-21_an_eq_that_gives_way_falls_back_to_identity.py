# typy-test: CLS-21
# typy-expect: accepted
class Money:
    def __init__(self, amount: int, currency: str) -> None:
        self.amount = amount
        self.currency = currency

    def __eq__(self, other: "Money | Tagged") -> bool:
        if not isinstance(other, Money) or other.currency != self.currency:
            return NotImplemented
        return self.amount == other.amount

    def __hash__(self) -> int:
        return self.amount


class Tagged:
    def __init__(self, amount: int) -> None:
        self.amount = amount

    def __eq__(self, other: "Money | Tagged") -> bool:
        print("Tagged.__eq__")
        if isinstance(other, Money):
            return other.amount == self.amount
        return NotImplemented


p = Money(1, "EUR")
q = Money(1, "EUR")
r = Money(3, "EUR")
d = Money(1, "USD")
print(p == q, p != q, p == r, p != r, p == p, p == d, p != d)
print(p in [r, q], p in [r, d], [r, q].index(p), [p, q, r, d].count(q))
print(p in {q}, {p: 1}[q], d in {p, q})
t = Tagged(1)
print(p == t, t == p, p != t, t == Tagged(1), t == t, d == t)
result = p.__eq__(d)
print(result, result is NotImplemented, result is not NotImplemented)
same = p.__eq__(q)
print(same, same is NotImplemented)
print(NotImplemented, repr(NotImplemented), type(NotImplemented).__name__)
