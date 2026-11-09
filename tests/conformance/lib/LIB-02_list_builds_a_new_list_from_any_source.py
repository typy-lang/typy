# typy-test: LIB-02
# typy-expect: accepted
xs = [3, 1, 2]
copy = list(xs)
copy.append(9)
print(xs, copy, list("h\u00e9llo"), list(reversed(xs)), list(enumerate("ab", 1)))
print(list(zip(xs, "abc", [True, False, True])), list(zip()), list(enumerate(zip(xs, xs))))
empty: list[str] = list()
print(empty, list(""), list([[1], [2]]))
