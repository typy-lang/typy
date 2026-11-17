# typy-test: LIB-21
# typy-expect: rejected E0201 at 5:17
import math

print(math.prod(["ab"]))
