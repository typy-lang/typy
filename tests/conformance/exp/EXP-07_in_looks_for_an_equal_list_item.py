# typy-test: EXP-07
# typy-expect: accepted
xs = [1, 2, 2 ** 70]
print(2 in xs, 5 in xs, 5 not in xs, 2 ** 70 in xs)
print("b" in ["a", "b"], [1] in [[1], [2]], 0.5 in [0.5], True in [False])
