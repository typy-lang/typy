# typy-test: LIB-03
# typy-expect: accepted
xs = [1, 2]
xs.extend([3, 4])
xs.extend(xs)
print(xs)
ys = xs.copy()
ys.reverse()
print(xs, ys, xs == ys)
alias = ys
ys.clear()
print(alias, len(alias), xs.reverse(), xs)
