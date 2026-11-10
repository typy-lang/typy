# typy-test: LIB-02
# typy-expect: accepted
xs = [3, -1, 2 ** 80, 7]
print(min(xs), max(xs), min(["b", "a", "c"]), max([[1, 2], [1, 3]]))
print(max([0.0, -0.0]), min([-0.0, 0.0]), max([True, False]))
empty: list[str] = []
print(max(empty, default="none"), min(xs, default=0))
print(min(empty))
