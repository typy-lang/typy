# typy-test: FUN-14
# typy-expect: accepted
from collections.abc import Callable

print(sorted(["10", "9", "100"], key=int), sorted([10, 9, 100], key=str))
parse: Callable[[str], int] = int
show: Callable[[int], str] = str
half: Callable[[int], float] = float
items: Callable[[str], list[str]] = list
print(parse("42") + 1, show(7), half(3), items("ab"), max(["3", "20"], key=int))
print(parse, show, type(parse), parse == show)
print(parse == int, int == parse, type(5) == int)
