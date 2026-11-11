# typy-test: TYP-06, DAT-07
# typy-expect: accepted
def f(x: list[int] | None) -> int:
    return 0


print(f([1]))
y = [2]
print(f(y))
items: list[int] | None = []
names: set[str] | None = {"a"}
counts: dict[str, int] | None = {}
print(items, names, counts, f([n for n in range(3)]))
