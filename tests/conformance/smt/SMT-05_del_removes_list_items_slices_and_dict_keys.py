# typy-test: SMT-05, STD-05
# typy-expect: accepted
numbers = list(range(10))
del numbers[0]
del numbers[-1]
print(numbers)
del numbers[1:3]
print(numbers)
del numbers[::2]
print(numbers)
del numbers[10:20], numbers[0]
print(numbers)
words = ["a", "b", "c", "d"]
del words[::-1]
print(words)
pairs = [(1, "one"), (2, "two"), (3, "three")]
del pairs[1]
print(pairs)
scores = {"ann": 3, "bob": 5}
del scores["ann"]
print(scores)
nested = {"k": [1, 2, 3]}
del nested["k"][1]
print(nested)
try:
    del numbers[100]
except IndexError as error:
    print("index:", error)
try:
    del scores["zed"]
except KeyError as error:
    print("key:", repr(error))
try:
    del words[::0]
except ValueError as error:
    print("value:", error)
del scores["bob"], scores["bob"]
