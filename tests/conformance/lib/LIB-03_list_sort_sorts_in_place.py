# typy-test: LIB-03
# typy-expect: accepted
xs = [3, 1, 2]
alias = xs
print(xs.sort(), alias)
xs.sort(reverse=True)
print(alias)
words = ["b", "C", "a"]
words.sort()
print(words)
