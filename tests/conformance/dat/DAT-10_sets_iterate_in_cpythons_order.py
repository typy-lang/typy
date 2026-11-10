# typy-test: DAT-10
# typy-expect: accepted
added: set[int] = set()
for value in [5, 1, 9, 33, 17, 1024, -3, 2 ** 70, -(2 ** 64)]:
    added.add(value)
print(added, len(added))
print(set([5, 1, 9, 33, 17, 1024, -3]), set(added), added.copy())
x = 33
print({x, 1, 9, 5}, {x}, {x, -x})
for item in added:
    print(item, end=" ")
print()
pairs = set([(1, 2), (3, 4), (1, 2), (0, 0), (5, -1), (2, 1)])
print(pairs, (1, 2) in pairs, (9, 9) in pairs)
floats = set([1.5, 0.1, -2.0, 1e300, 0.0, -0.0, float("inf"), 2.5])
print(floats, 1.5 in floats, -0.0 in floats)
print(set([True, False, True]), set([None, None]), set([b"x", b"x"]))
nested = set([(1, (2, 3)), (1, (2, 3)), (4, (5, 6))])
print(nested, (4, (5, 6)) in nested)
grown: set[int] = set()
for value in range(2000):
    grown.add(value * 7919 % 10007)
for value in range(0, 2000, 3):
    grown.discard(value * 7919 % 10007)
print(len(grown), list(grown)[:8], list(grown)[-8:])
popped: list[int] = []
for _ in range(6):
    popped.append(grown.pop())
print(popped, len(grown))
