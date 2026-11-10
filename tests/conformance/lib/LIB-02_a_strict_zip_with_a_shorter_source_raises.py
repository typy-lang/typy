# typy-test: LIB-02
# typy-expect: accepted
words = ["a", "b"]
for n, s, m in zip([1, 2, 3], words, [7, 8, 9], strict=len(words) > 1):
    print(n, s, m)
