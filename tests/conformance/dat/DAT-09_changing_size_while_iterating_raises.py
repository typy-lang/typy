# typy-test: DAT-09, EXC-02
# typy-expect: accepted
d = {1: 1, 2: 2}
for k in d:
    d[k + 10] = 0
