# typy-test: EXP-17, SMT-02
# typy-expect: accepted
first, *rest = [1, 2, 3]
print(first, rest)
*init, last = (1, 2, 3)
print(init, last)
head, *middle, tail = "abcd"
print(head, middle, tail)
one, *none = [1]
print(one, none)
[p, *q] = range(3)
print(p, q)
for x, *ys in [[1, 2, 3], [4]]:
    print(x, ys)
