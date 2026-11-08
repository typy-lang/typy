# typy-test: DAT-07, MEM-02
# typy-expect: accepted
xs = [3, 1, 2]
alias = xs
alias.append(4)
xs[0] = 30
print(xs, alias, alias == xs, len(xs))
grid = [[1, 2], [3]]
row = grid[1]
row.append(4)
print(grid, grid[1][1])
floats = [1.5, -0.0, 2.25, 1e300 * 10.0]
print(floats, [True, False], ["it's", "b"], [2 ** 70, -1])


def make() -> list[str]:
    return ["made", "here"]


def ratios() -> list[float]:
    return [0.5, 0.25]


print(make()[0], make()[-1], ratios()[1], [1] != [2], [1] != [1])
