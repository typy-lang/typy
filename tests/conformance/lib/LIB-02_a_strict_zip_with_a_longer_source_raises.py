# typy-test: LIB-02
# typy-expect: accepted
for n, m in zip([1, 2], [1, 2], strict=True):
    print(n, m)
for n, m, k in zip([1], [1], [1, 2], strict=True):
    print(n, m, k)
