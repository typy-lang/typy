# typy-test: LIB-08, MOD-04
# typy-expect: accepted
import math
import sys
from math import pi

print(math.floor(pi), len(sys.argv) > 0)
