# typy-test: LIB-02
# typy-expect: accepted
xs = [5, 3, 9, 1, 3, 2 ** 70, -4]
print(sorted(xs), sorted(xs, reverse=True), xs)
words = ["pear", "Apple", "fig", "apple", ""]
print(sorted(words), sorted(words, reverse=False))
zeros = [0.0, -0.0, 1.5, -0.0, 0.0, -2.5]
print(sorted(zeros), sorted(zeros, reverse=True))
grid = [[3, 1], [2], [3, 0], []]
print(sorted(grid))
numbers: list[int] = []
for step in range(500):
    numbers.append((step * 7919) % 1000)
ordered = sorted(numbers)
print(ordered[:5], ordered[-3:], len(ordered), sorted(numbers, reverse=True)[:3])
