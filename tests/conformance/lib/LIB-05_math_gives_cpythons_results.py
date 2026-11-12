# typy-test: LIB-05
# typy-expect: accepted
import math
from math import sqrt, pi, floor, gcd as divisor
import math as m

print(math, m is math, math.pi, math.e, math.tau, math.inf, -math.inf, math.nan)
print(sqrt(2), sqrt(16), math.sqrt(10**20), math.cbrt(27), math.cbrt(2))
print(math.floor(-2.5), math.ceil(-2.5), math.trunc(-2.5), floor(7), math.floor(True), math.ceil(10**30))
print(math.log(8, 2), math.log(100), math.log2(1024), math.log10(10**400))
print(math.exp(1), math.expm1(1e-10), math.log1p(1e-10), math.pow(2, 0.5), math.pow(-2, 3))
print(math.sin(pi / 6), math.cos(0), math.tan(1), math.atan2(1, -1), math.asin(1), math.acos(-1))
print(math.sinh(1), math.cosh(1), math.tanh(1), math.asinh(1), math.acosh(2), math.atanh(0.5))
print(math.degrees(pi), math.radians(180), math.hypot(3, 4), math.hypot(), math.hypot(1, 2, 2), math.hypot(-5))
print(math.fabs(-3), math.copysign(3, -0.0), math.fmod(-7, 3), math.remainder(7, 2), math.remainder(5, 3))
print(math.frexp(8.0), math.frexp(0.0), math.modf(-2.5), math.modf(math.inf), math.ldexp(0.5, 4))
print(math.isfinite(1), math.isinf(math.inf), math.isnan(math.nan), math.isclose(1, 1.000000001), math.isclose(1, 1.1, rel_tol=0.2))
print(math.isclose(0, 1e-12, abs_tol=1e-11), math.ulp(1.0), math.nextafter(1, 2), math.nextafter(0, -1, steps=3))
print(math.factorial(20), math.factorial(30), math.isqrt(10**40 + 1), math.comb(50, 25), math.perm(5), math.perm(5, 2))
print(divisor(), divisor(12), math.gcd(-12, 18, 30), math.lcm(4, 6, 10), math.lcm(), math.lcm(0, 5))
print(math.erf(1), math.erfc(1), math.exp2(10))
k: int | None = None
print(math.perm(4, k), math.nextafter(1.0, 0.0, steps=k))
for value in [-1.0, 0.0, 2.0, math.inf]:
    try:
        print(math.sqrt(value), math.log(value), math.acos(value))
    except ValueError:
        print("ValueError")
for big in [0, -3, 10**400]:
    try:
        print(math.log(big))
    except ValueError:
        print("ValueError")
try:
    print(math.exp(1000))
except OverflowError:
    print("OverflowError")
try:
    print(math.floor(math.inf))
except OverflowError:
    print("OverflowError")
try:
    print(math.factorial(-1))
except ValueError:
    print("ValueError")
try:
    print(math.log(2, 1))
except ZeroDivisionError:
    print("ZeroDivisionError")
try:
    print(math.sqrt(10**400))
except OverflowError:
    print("OverflowError")
