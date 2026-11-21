# typy-test: LIB-23
# typy-expect: accepted
def double(value: int) -> int:
    return 2 * value


words = ["1", "22", "333"]
print(list(map(double, [1, 2, 3])), list(map(int, words)), list(map(str.upper, ["a"])))
print(list(map(lambda x: x + 1, [1, 2])), sum(map(double, range(4))), list(map(len, words)))
print(list(filter(lambda x: x > 1, [1, 2, 3])), list(filter(str.isdigit, ["1", "a"])))
print(list(filter(None, [0, 1, 2, 0])), list(map(lambda a, b: a * b, [1, 2, 3], [4, 5])))
mapped = map(int, ["1", "x", "3"])
print(next(mapped))
try:
    next(mapped)
except ValueError as error:
    print("error", error)
print(next(mapped))
for value in map(double, (5, 6)):
    print(value)
print(type(map(str, [1])), type(filter(None, [1])))
print(repr(map(str, [1]))[:15], list(map(str, map(double, [7]))))
