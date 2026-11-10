# typy-test: LIB-02
# typy-expect: accepted
xs = [10, 20, 30]
for i, x in enumerate(xs):
    print(i, x)
for pair in enumerate("ab", 5):
    print(pair)
for i, x in enumerate(xs, start=-(2 ** 70)):
    print(i, x)
grow = [1]
for i, x in enumerate(grow):
    if x < 3:
        grow.append(x + 1)
    print(i, x)
