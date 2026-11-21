# typy-test: DAT-19
# typy-expect: accepted
flags = [True, False, True]
print(True + 1, True - True, True * 3, True / 2, -True, +False, 2 ** True, 7 // True, 7 % True)
print(True & False, True | False, True ^ True, True & 3, 6 | True, abs(True), round(True), round(False, 2))
print(sum(flags), sum(flags, 10))
total = 0
total += True
print(total, 1.5 + True, True == 1, True + 0.5, True < 2, 1.0 == True)
values: list[object] = [1, True, 2.5]
count = 0
for value in values:
    if isinstance(value, int):
        count += value
print(count)
