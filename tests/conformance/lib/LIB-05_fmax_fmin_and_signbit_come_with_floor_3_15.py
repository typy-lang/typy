# typy-test: LIB-05
# typy-expect: accepted
# typy-python: 3.15
import math

print(math.fmax(1, 2.5), math.fmin(-0.0, 0.0), math.signbit(-0.0), math.isnormal(1e-310), math.issubnormal(1e-310))
