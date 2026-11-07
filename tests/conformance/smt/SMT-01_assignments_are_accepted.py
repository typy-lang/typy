# typy-test: SMT-01
# typy-expect: accepted
a = b = 2
c: int = 3
c += a
c *= b
c -= 1
c //= 2
c **= 2
d: str = "x"
d *= 3
print(a, b, c, d)
