# typy-test: LIB-15
# typy-expect: rejected E0201 at 6:17
import math

mixed: list[int | float] = [1, 2.5]
print(math.fsum(mixed))
