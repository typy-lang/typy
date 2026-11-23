# typy-test: DAT-11, MEM-02
# typy-expect: accepted
# An item of one word is read and written in place while the index is an inline int inside
# the list; a negative index counts from the end, and anything else, or an item that needs a
# reference count, goes through the runtime, which raises IndexError where Python does.
numbers = [5, 6, 7]
for i in [0, 2, -1, -3]:
    numbers[i] = numbers[i] * 2 + numbers[-1]
print(numbers, numbers[-2])

for index in [3, -4, 2**70, -(2**70)]:
    try:
        print(numbers[index])
    except IndexError as error:
        print("read", index, type(error).__name__)
    try:
        numbers[index] = 0
    except IndexError as error:
        print("write", index, type(error).__name__)

large = [2**80, 1]
large[1] = 2**90
large[0] = 3
large[1] = large[1] // 2**89
print(large)

reals = [0.5, 1.5, -2.0]
reals[1] = reals[0] / 4
reals[-1] += reals[1]
print(reals)

flags = [True, False]
flags[1] = flags[0]
print(flags)

words = ["a", "b"]
words[0] = words[1] + "c"
words[-1] = words[0] * 2
print(words, words[-2])

pairs = [(1, "x"), (2, "y")]
pairs[0] = pairs[1]
print(pairs[0], pairs[-1])

maybe: list[int | None] = [None, 3]
maybe[0] = maybe[1]
maybe[1] = None
print(maybe)

grid = [[1, 2], [3, 4]]
grid[1][0] = grid[0][1] * 10
row = grid[0]
grid[0] = [9]
print(grid, row)

items: tuple[int, ...] = (10, 20, 30)
for position in [-1, 0, 3]:
    try:
        print(items[position])
    except IndexError as error:
        print("tuple", position, type(error).__name__)
