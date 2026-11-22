# typy-test: EXP-22
# typy-expect: rejected E0201 at 6:18
import math

values = [1.5]
print(math.floor(*values))
