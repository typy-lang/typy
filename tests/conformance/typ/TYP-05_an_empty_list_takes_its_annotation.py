# typy-test: TYP-05
# typy-expect: accepted
names: list[str] = []
names.append("a")


def none() -> list[float]:
    return []


grid: list[list[int]] = [[], [1]]
print(names, none(), grid)
