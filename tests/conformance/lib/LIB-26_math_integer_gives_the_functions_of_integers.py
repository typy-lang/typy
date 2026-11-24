# typy-test: LIB-26
# typy-expect: accepted
# typy-python: 3.15
# math.integer gives comb, factorial, gcd, isqrt, lcm and perm, read through the module, an
# alias or by from.
import math.integer
import math.integer as mi
from math import integer
from math.integer import comb, isqrt

print(math.integer.gcd(12, 18), mi.lcm(4, 6), integer.factorial(5))
print(comb(5, 2), isqrt(17), mi.perm(5, 2), math.integer.factorial(20))
print(mi.gcd(0, 0), mi.lcm(3, 5, 7), integer.isqrt(10**40))
