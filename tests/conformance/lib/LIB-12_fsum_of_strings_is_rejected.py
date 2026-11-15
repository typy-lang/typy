# typy-test: LIB-12
# typy-expect: rejected E0404 at 5:17
import math

print(math.fsum(["0.5"]))
