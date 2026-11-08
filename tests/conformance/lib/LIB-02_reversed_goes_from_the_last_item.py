# typy-test: LIB-02
# typy-expect: accepted
xs = [1, 2, 3, 4]
for x in reversed(xs):
    print(x)
for letter in reversed("h\u00e9llo"):
    print(letter, end="")
print()
for x in reversed(xs):
    xs.clear()
    print(x)
for x in reversed(xs):
    print("never")
else:
    print("empty")
