# typy-test: LIB-20
# typy-expect: accepted
from collections.abc import Reversible


def back(items: Reversible[int]) -> list[int]:
    return list(reversed(items))


def pairs(items: Reversible[tuple[str, int]]) -> list[tuple[str, int]]:
    return list(reversed(items))


table = {"a": 1, "b": 2}
print(back(table.values()), pairs(table.items()), back([]))
grow = {1: "x", 2: "y"}
try:
    for key in reversed(grow):
        grow[key + 10] = "z"
except RuntimeError as error:
    print("RuntimeError:", error)
print(list(reversed(table)), list(reversed(table.values())))
