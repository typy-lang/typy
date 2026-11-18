# typy-test: FUN-08
# typy-expect: rejected E0201 at 4:25
words = ["bb", "a"]
print(sorted(words, key=len))
