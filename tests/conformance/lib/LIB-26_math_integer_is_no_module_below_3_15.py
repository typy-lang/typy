# typy-test: LIB-26
# typy-expect: rejected E0206 at 4:1
# Below the Python floor 3.15, `math` is no package, as in CPython 3.14.
import math.integer
