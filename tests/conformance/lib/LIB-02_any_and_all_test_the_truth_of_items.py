# typy-test: LIB-02
# typy-expect: accepted
nothing: list[int] = []
print(any([0, 0, 3]), all([1, 2, 0]), any(nothing), all(nothing))
print(any(["", "a"]), all(["a", ""]), any([[], [0]]), all([[1], []]))
print(any([0.0, -0.0]), all([float("nan")]), any([False, True]), all([True]))
