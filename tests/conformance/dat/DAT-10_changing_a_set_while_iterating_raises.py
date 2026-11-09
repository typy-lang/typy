# typy-test: DAT-10, EXC-02
# typy-expect: accepted
values = set([1, 2, 3])
for value in values:
    print(value)
    if value == 2:
        values.add(10)
