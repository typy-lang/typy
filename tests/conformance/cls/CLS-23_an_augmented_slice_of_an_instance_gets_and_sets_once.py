# typy-test: CLS-23
# typy-expect: accepted
class Grid:
    def __init__(self) -> None:
        self.cells = [1, 2, 3, 4]

    def __getitem__(self, part: slice) -> list[int]:
        print("get", part)
        return self.cells[part]

    def __setitem__(self, part: slice, values: list[int]) -> None:
        print("set", part, values)
        self.cells[part.start:part.stop] = values


def grid() -> Grid:
    print("grid")
    return made


made = Grid()
grid()[1:3] += [9]
print(made.cells, made[:], made[::-1])
items = [1, 2, 3, 4]
s = slice(1, 3)
print(items[s], items[slice(None)], "text"[slice(-2, None)])
