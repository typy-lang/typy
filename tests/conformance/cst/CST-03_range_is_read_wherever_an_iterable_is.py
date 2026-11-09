# typy-test: CST-03, LIB-02
# typy-expect: accepted
print(list(range(5)), list(range(2, 9, 3)), list(range(10, 0, -4)), list(range(3, 3)))
print(list(reversed(range(5))), list(reversed(range(0, 10, 3))), list(reversed(range(10, 0, -3))), list(reversed(range(5, 5))))
print(sum(range(101)), sorted(range(3), reverse=True), max(range(7)), set(range(4)), tuple(range(2)))
for i, value in enumerate(range(10, 13)):
    print(i, value, end="; ")
print()
for a, b in zip(range(3), reversed(range(3))):
    print(a, b, end="; ")
print()
big = 2 ** 70
print(list(range(big, big + 3)), list(reversed(range(-big, -big + 2))))
n = -2
for x in reversed(range(0, 6, n)):
    print(x)
s: set[int] = set()
s.update(range(3), reversed(range(5, 7)))
print(s, dict(zip(range(3), "abc")))
step = 0
print(list(range(1, 2, step)))
