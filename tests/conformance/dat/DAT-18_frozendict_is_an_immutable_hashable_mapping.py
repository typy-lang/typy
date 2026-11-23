# typy-test: DAT-18
# typy-expect: accepted
# typy-python: 3.15
# frozendict is an immutable, hashable mapping with dict's reading methods, | and its text,
# made from what dict takes, and read as a dict where CPython reads any mapping.

prices: frozendict[str, int] = frozendict({"tea": 3, "cake": 5})
print(prices, prices["tea"], len(prices), "tea" in prices, list(prices))
print(prices.get("milk"), prices.get("cake"), prices.keys(), prices.values(), prices.items())
same = frozendict({"cake": 5, "tea": 3})
print(prices == same, hash(prices) == hash(same), prices == {"tea": 3, "cake": 5})
print(prices | {"milk": 2}, prices | same, sorted(prices), isinstance(prices, frozendict))
print(prices.copy() is prices, {prices: "menu"}[same], type(prices))
lists: frozendict[str, list[int]] = frozendict({"a": [1]})
print(lists)
try:
    hash(lists)
except TypeError as error:
    print("TypeError", error)
try:
    prices["milk"]
except KeyError as error:
    print("KeyError", error)
print(dict(prices), {**prices, "x": 9}, list(reversed(prices)), frozendict([("a", 1)]), frozendict(a=1, b=2))
merged: dict[str, int] = {"x": 1}
merged.update(prices)
merged |= same
print(merged, {"x": 1} | prices, prices.get("z", 0) + 1, frozendict(prices) == prices)
empty: frozendict[str, int] = frozendict()
print(empty, len(empty), frozendict({"k": 1}, k=2))


def total(**amounts: int) -> int:
    return sum(amounts.values())


print(total(**prices), [key for key in prices], type(prices), frozendict)
