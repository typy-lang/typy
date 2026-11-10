# typy-test: LIB-03
# typy-expect: accepted
xs = [1, 2, 3, 4, 5]
print(xs.pop(), xs.pop(0), xs.pop(-2), xs)
xs.insert(1, 7)
xs.insert(-1, 8)
xs.insert(100, 9)
xs.insert(-100, 6)
print(xs)
xs.remove(7)
print(xs)
words = ["a", "b", "a"]
words.remove("a")
print(words, words.pop(), words)
xs.remove(42)
