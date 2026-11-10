# typy-test: SMT-02
# typy-expect: accepted
a, b = 1, 2
a, b = b, a
print(a, b)
x, (y, (z, w)), flag = (1, ("x", (2.5, None)), True)
print(x, y, z, w, flag)
first, second = ("x" + "y", "z" * 3)
print(first, second)
p = q = (1, "shared")
print(p, q)


def split(pair: tuple[int, str]) -> str:
    number, text = pair
    return text * number


print(split((3, "ab")))
