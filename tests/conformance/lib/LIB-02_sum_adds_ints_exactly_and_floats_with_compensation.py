# typy-test: LIB-02
# typy-expect: accepted
print(sum([1, 2, 3]), sum([2 ** 64, 2 ** 64], 5), sum([4], start=-4))
print(sum([0.1] * 10, 0.0), sum([1e100, 1.0, -1e100], 0.0), sum([0.1, 0.2, 0.3], 0.0))
print(sum([1e308, 1e308], 0.0), sum([-0.0, -0.0], -0.0), sum([2.5], start=1.0))
empty: list[float] = []
print(sum(empty, -0.0), sum(empty, 0.0))
