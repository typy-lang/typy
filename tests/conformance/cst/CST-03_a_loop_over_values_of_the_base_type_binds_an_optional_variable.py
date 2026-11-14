# typy-test: CST-03
# typy-expect: accepted
for value in [None, "a"]:
    print(value)
for value in ["x", "y"]:
    print(value)
number: int | None = None
for number in range(2):
    print(number + 1)
print(number)
