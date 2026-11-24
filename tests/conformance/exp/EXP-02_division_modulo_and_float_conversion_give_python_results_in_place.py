# typy-test: EXP-02, EXP-03, DAT-01
# typy-expect: accepted
# `//`, `%` and `float()` of inline values, and `/` of floats, run in place where they cannot
# raise, and in the runtime for a large integer, a zero divisor or `-2**62 // -1`, whose
# quotient leaves the inline range; the results are the same either way.
import math

EDGE = 2**62
VALUES = [0, 1, -1, 2, -2, 3, -3, 7, -7, EDGE - 1, EDGE - 2, -EDGE, -EDGE + 1, 2**31, 2**64 + 1, -(2**70)]

for a in VALUES:
    for b in VALUES:
        try:
            print(a, b, a // b, a % b, (a // b) * b + a % b == a)
        except ZeroDivisionError as error:
            print(a, b, type(error).__name__)

for n in [0, 1, -1, 2**53, 2**53 + 1, 2**53 + 3, -(2**53) - 1, EDGE - 1, -EDGE, 2**62 - 3, 10**30]:
    print(n, float(n), float(-n))

FLOATS = [1.0, -1.0, 0.5, 3.0, 1e308, 5e-324, math.inf, -math.inf, math.nan, 0.0, -0.0]
for x in FLOATS:
    for y in FLOATS:
        try:
            print(x, y, x / y)
        except ZeroDivisionError as error:
            print(x, y, type(error).__name__)
