# typy-test: DAT-12
# typy-expect: accepted
t = (1, "two", 3.0, None, [4])
print(t[1:3], t[::-1], t[:-2], t[10:], t[::2], t[-100:1], t[3:1:-1], t[:])
nested = ((1, 2), (3, 4))
print(nested[1:], nested[::-1][0][1])
x, y = t[1:3]
print(x, y)
