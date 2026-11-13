# typy-test: CLS-07
# typy-expect: accepted
class Grid:
    def __init__(self, width: int) -> None:
        self.width = width
        self.cells: dict[int, str] = {}

    def __len__(self) -> int:
        return len(self.cells)

    def __getitem__(self, key: int) -> str:
        return self.cells.get(key, ".")

    def __setitem__(self, key: int, value: str) -> None:
        self.cells[key] = value

    def __delitem__(self, key: int) -> None:
        del self.cells[key]

    def __contains__(self, key: int) -> bool:
        return key in self.cells


class Flag:
    def __init__(self, on: bool) -> None:
        self.on = on

    def __bool__(self) -> bool:
        return self.on


class Broken:
    def __len__(self) -> int:
        return -1


grid = Grid(3)
print(len(grid), bool(grid), grid[0])
grid[0] = "x"
grid[4] = "y"
print(len(grid), grid[0], grid[4], 4 in grid, 5 in grid, 5 not in grid)
del grid[0]
print(len(grid), 0 in grid, "empty" if not Grid(1) else "full")
flags = [Flag(True), Flag(False)]
for flag in flags:
    if flag:
        print("on")
    else:
        print("off")
print(bool(Flag(False)), not Flag(True))
try:
    print(len(Broken()))
except ValueError as error:
    print("error:", error)
try:
    if Broken():
        print("true")
except ValueError as error:
    print("error:", error)

flags = [Flag(False), Flag(True)]
print(any(flags), all(flags), any([Flag(False)]), all([Flag(True)]))
