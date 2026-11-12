# typy-test: LIB-05
# typy-expect: accepted
# typy-python: 3.13
import math

print(math.fma(2.0, 3.0, 1.0), math.fma(0.1, 10, -1))
