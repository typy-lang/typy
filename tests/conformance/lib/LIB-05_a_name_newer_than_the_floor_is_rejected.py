# typy-test: LIB-05
# typy-expect: rejected E0425 at 5:12
import math

print(math.fma(2.0, 3.0, 1.0))
