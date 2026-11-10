# typy-test: DAT-09, EXC-02
# typy-expect: accepted
d = {1: 1, 2: 2, 3: 3}
for k in d:
    print(k)
    if k == 1:
        d.pop(2)
        d[5] = 5
