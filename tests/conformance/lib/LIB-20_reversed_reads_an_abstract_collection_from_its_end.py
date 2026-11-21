# typy-test: LIB-20
# typy-expect: accepted
from collections.abc import Reversible, Sequence


def backwards(values: Sequence[int]) -> list[int]:
    return [value for value in reversed(values)]


def back(items: Reversible[str]) -> str:
    return "".join(reversed(items))


table = {"a": 1, "b": 2, "c": 3}
print(backwards([1, 2, 3]), backwards((4, 5)), backwards(range(3)), backwards(b"ab"), backwards(range(10, 0, -3)))
print(back("xyz"), back(["p", "q"]), back(table), back(table.keys()), back(("m", "n")))
for value in reversed([7, 8]):
    print(value)
