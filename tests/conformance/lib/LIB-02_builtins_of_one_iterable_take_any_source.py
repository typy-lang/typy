# typy-test: LIB-02
# typy-expect: accepted
xs = [3, 1, 2]
print(sorted("banana"), sorted(zip([2, 1], "ab")), max("hello"), min(enumerate("cab")))
print(any(""), all("abc"), sum(reversed(xs)), sorted(reversed(xs), reverse=True))
print(max(zip([1, 2], [3, 0])), min("", default="none"))
