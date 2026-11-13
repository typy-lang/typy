# typy-test: DAT-13, STD-05
# typy-expect: accepted
letters = ["a", "b", "c", "d", "e"]
letters[1:3] = ["X"]
print(letters)
letters[:0] = ["start", "again"]
print(letters)
letters[::2] = ["1", "2", "3"]
print(letters)
letters[:] = []
print(letters)
numbers = [1, 2, 3]
numbers[1:] = numbers
print(numbers)
grid = [[0, 0], [1, 1]]
grid[0][:] = [5, 6]
print(grid)
pairs = [(1, "a"), (2, "b")]
pairs[1:] = [(3, "c"), (4, "d")]
print(pairs)
try:
    numbers[::2] = [0]
except ValueError as error:
    print(error)
numbers[::-1] = [9, 8]
