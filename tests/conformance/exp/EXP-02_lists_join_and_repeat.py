# typy-test: EXP-02
# typy-expect: accepted
xs = [1, 2]
print(xs + [3], [3] + xs, xs + xs, xs * 3, 2 * xs, xs * 0, xs * -5)
grid = [[0]] * 3
grid[0].append(1)
print(grid)
print(["a"] * (2 ** 70))
