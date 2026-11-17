# typy-test: TYP-14
# typy-expect: accepted
def shape(kind: int) -> tuple[()] | tuple[str] | tuple[str, list[int]]:
    if kind == 0:
        return ()
    if kind == 1:
        return ("one",)
    return ("two", [1, 2])


def describe(value: tuple[int, int] | tuple[float]) -> list[str]:
    return [str(item) for item in value]


for kind in range(3):
    value = shape(kind)
    for item in value:
        print(kind, item)
    print(list(value), len([item for item in value]))
print(describe((1, 2)), describe((2.5,)))
pair: tuple[int, int] | tuple[int] = (3, 4)
print(sum(pair), sorted(pair), max(pair))
