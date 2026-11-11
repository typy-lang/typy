# typy-test: DAT-16, CST-03
# typy-expect: accepted
ages = {"ada": 36, "alan": 41, "grace": 85, "linus": 54}
del ages["alan"]
ages["edsger"] = 72
for name in reversed(ages):
    print(name, end=" ")
print()
print(list(reversed(ages.keys())), list(reversed(ages.values())), list(reversed(ages.items())))
print([k for k in reversed(ages)], sorted(reversed(ages)), sum(reversed(ages.values())))
for index, (name, age) in enumerate(reversed(ages.items())):
    print(index, name, age)
for pair in zip(reversed(ages), ages):
    print(pair)
empty: dict[int, int] = {}
print(list(reversed(empty)), list(reversed(empty.items())))
numbers = {n: n for n in range(20)}
for n in list(numbers):
    if n % 3:
        del numbers[n]
print(list(reversed(numbers)))
counts = {1: 1, 2: 2}
try:
    for key in reversed(counts):
        counts[key + 10] = 0
except RuntimeError as error:
    print("RuntimeError", error)
same = {1: 1, 2: 2, 3: 3}
for key in reversed(same):
    if key == 3:
        del same[1]
        same[9] = 9
    print(key)
