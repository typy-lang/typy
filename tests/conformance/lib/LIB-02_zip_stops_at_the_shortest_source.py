# typy-test: LIB-02
# typy-expect: accepted
xs = [10, 20, 30]
for n, s in zip(xs, "abcd"):
    print(n, s)
for n, f, s in zip(xs, [1.5, 2.5], ["x", "y", "z"]):
    print(n, f, s)
for t in zip():
    print(t)
else:
    print("nothing")
for i, (n, m) in enumerate(zip(xs, reversed(xs))):
    print(i, n + m)
for (i, s), n in zip(enumerate("ab"), xs):
    print(i, s, n)
for n, m in zip([1, 2], [1, 2, 3], strict=False):
    print(n, m)
