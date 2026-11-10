# typy-test: FUN-05
# typy-expect: rejected E0201 at 4:21
words = ["bb", "a"]
print(sorted(words, key=len))
