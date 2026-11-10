# typy-test: DAT-12
# typy-expect: accepted
xs = [0, 1, 2, 3, 4, 5]
print(xs[1:4], xs[:2], xs[4:], xs[:], xs[::2], xs[1::2])
print(xs[::-1], xs[4:1:-1], xs[-2:], xs[:-2], xs[-100:100], xs[5:1])
print(xs[2 ** 80 :], xs[: -(2 ** 80)], xs[:: -(2 ** 80)])
copy = xs[:]
copy.append(6)
print(xs, copy)
nested = [[1], [2]]
part = nested[:1]
part[0].append(9)
print(nested)
