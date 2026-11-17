# typy-test: DAT-12
# typy-expect: accepted
items = [1, 2, 3, 4]
s = slice(1, 3)
items[s] = [5]
print(items)
del items[slice(None, None, 2)]
print(items)
words = ["a", "b", "c"]
words[slice(1, None)] = []
print(words)
